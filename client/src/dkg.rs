use std::borrow::Borrow;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Debug, Formatter};
use std::string::ToString;
use std::sync::Arc;

use bincode;
use failure::Fail;
use rand::{self, Rng};
use serde::{Deserialize, Serialize};
use std::hash::Hash;
use threshold_crypto::{
    error::Error as CryptoError,
    pairing::{CurveAffine, Field},
    poly::{BivarCommitment, BivarPoly, Poly},
    serde_impl::FieldWrap,
    Fr, G1Affine, PublicKeySet, SecretKeyShare,
};
/// A peer node's unique identifier.
pub trait NodeIdT: Eq + Ord + Clone + Debug + Hash + Send + Sync {}
impl<N> NodeIdT for N where N: Eq + Ord + Clone + Debug + Hash + Send + Sync {}

/// A cryptographic key that allows decrypting messages that were encrypted to the key's owner.
pub trait SecretKey {
    /// The decryption error type.
    type Error: ToString;

    /// Decrypts a ciphertext.
    fn decrypt(&self, ct: &[u8]) -> Result<Vec<u8>, Self::Error>;
}

/// A cryptographic public key that allows encrypting messages to the key's owner.
pub trait PublicKey {
    /// The encryption error type.
    type Error: ToString;
    /// The corresponding secret key type. The secret key is known only to the key's owner.
    type SecretKey: SecretKey;

    /// Encrypts a message to this key's owner and returns the ciphertext.
    fn encrypt<M: AsRef<[u8]>, R: Rng>(&self, msg: M, rng: &mut R) -> Result<Vec<u8>, Self::Error>;
}

impl SecretKey for threshold_crypto::SecretKey {
    type Error = bincode::Error;

    fn decrypt(&self, ct: &[u8]) -> Result<Vec<u8>, bincode::Error> {
        self.decrypt(&bincode::deserialize(ct)?)
            .ok_or_else(|| bincode::ErrorKind::Custom("Invalid ciphertext.".to_string()).into())
    }
}

impl PublicKey for threshold_crypto::PublicKey {
    type Error = bincode::Error;
    type SecretKey = threshold_crypto::SecretKey;

    fn encrypt<M: AsRef<[u8]>, R: Rng>(
        &self,
        msg: M,
        rng: &mut R,
    ) -> Result<Vec<u8>, bincode::Error> {
        bincode::serialize(&self.encrypt_with_rng(rng, msg))
    }
}

/// A map assigning to each node ID a public key, wrapped in an `Arc`.
pub type PubKeyMap<N, PK = threshold_crypto::PublicKey> = Arc<BTreeMap<N, PK>>;

/// Returns a `PubKeyMap` corresponding to the given secret keys.
///
/// This is mostly useful for setting up test networks.
pub fn to_pub_keys<'a, I, B, N: NodeIdT + 'a>(sec_keys: I) -> PubKeyMap<N>
where
    B: Borrow<N>,
    I: IntoIterator<Item = (B, &'a threshold_crypto::SecretKey)>,
{
    let to_pub = |(id, sk): I::Item| (id.borrow().clone(), sk.public_key());
    Arc::new(sec_keys.into_iter().map(to_pub).collect())
}

/// A local error while handling an `Ack` or `Part` message, that was not caused by that message
/// being invalid.
#[derive(Clone, Eq, PartialEq, Debug, Fail)]
pub enum Error {
    /// Error creating `SyncKeyGen`.
    #[fail(display = "Error creating SyncKeyGen: {}", _0)]
    Creation(CryptoError),
    /// Error generating keys.
    #[fail(display = "Error generating keys: {}", _0)]
    Generation(CryptoError),
    /// Unknown sender.
    #[fail(display = "Unknown sender")]
    UnknownSender,
    /// Failed to serialize message.
    #[fail(display = "Serialization error: {}", _0)]
    Serialize(String),
    /// Failed to encrypt message parts for a peer.
    #[fail(display = "Encryption error: {}", _0)]
    Encrypt(String),
}

impl From<bincode::Error> for Error {
    fn from(err: bincode::Error) -> Error {
        Error::Serialize(format!("{:?}", err))
    }
}

impl Error {
    fn encrypt<E: ToString>(err: E) -> Error {
        Error::Encrypt(err.to_string())
    }
}

/// A submission by a validator for the key generation. It must to be sent to all participating
/// nodes and handled by all of them, including the one that produced it.
///
/// The message contains a commitment to a bivariate polynomial, and for each node, an encrypted
/// row of values. If this message receives enough `Ack`s, it will be used as summand to produce
/// the the key set in the end.
#[derive(Deserialize, Serialize, Clone, Hash, Eq, PartialEq)]
pub struct Part(BivarCommitment, Vec<Vec<u8>>);

impl Debug for Part {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Part")
            .field(&format!("<degree {}>", self.0.degree()))
            .field(&format!("<{} rows>", self.1.len()))
            .finish()
    }
}

/// A confirmation that we have received and verified a validator's part. It must be sent to
/// all participating nodes and handled by all of them, including ourselves.
///
/// The message is only produced after we verified our row against the commitment in the `Part`.
/// For each node, it contains one encrypted value of that row.
#[derive(Deserialize, Serialize, Clone, Hash, Eq, PartialEq)]
pub struct Ack(u64, Vec<Vec<u8>>);

impl Debug for Ack {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Ack")
            .field(&self.0)
            .field(&format!("<{} values>", self.1.len()))
            .finish()
    }
}

/// The information needed to track a single proposer's secret sharing process.
#[derive(Debug, PartialEq, Eq)]
struct ProposalState {
    /// The proposer's commitment.
    commit: BivarCommitment,
    /// The verified values we received from `Ack` messages.
    values: BTreeMap<u64, Fr>,
    /// The nodes which have acked this part, valid or not.
    acks: BTreeSet<u64>,
}

impl ProposalState {
    /// Creates a new part state with a commitment.
    fn new(commit: BivarCommitment) -> ProposalState {
        ProposalState {
            commit,
            values: BTreeMap::new(),
            acks: BTreeSet::new(),
        }
    }

    /// Returns `true` if at least `threshold + 1` nodes have acked.
    fn is_complete(&self, threshold: usize) -> bool {
        self.acks.len() > threshold
    }
}

/// The outcome of handling and verifying a `Part` message.
pub enum PartOutcome {
    /// The message was valid: the part of it that was encrypted to us matched the public
    /// commitment, so we can multicast an `Ack` message for it. If we are an observer or we have
    /// already handled the same `Part` before, this contains `None` instead.
    Valid(Option<Ack>),
    /// The message was invalid: We now know that the proposer is faulty, and dont' send an `Ack`.
    Invalid(PartFault),
}

/// The outcome of handling and verifying an `Ack` message.
pub enum AckOutcome {
    /// The message was valid.
    Valid,
    /// The message was invalid: The sender is faulty.
    Invalid(AckFault),
}

/// A synchronous algorithm for dealerless distributed key generation.
///
/// It requires that all nodes handle all messages in the exact same order.
#[derive(Debug)]
pub struct SyncKeyGen<N, PK: PublicKey = threshold_crypto::PublicKey> {
    /// Our node ID.
    our_id: N,
    /// Our node index.
    our_idx: Option<u64>,
    /// Our secret key.
    sec_key: PK::SecretKey,
    /// The public keys of all nodes, by node ID.
    pub_keys: PubKeyMap<N, PK>,
    /// Proposed bivariate polynomials.
    parts: BTreeMap<u64, ProposalState>,
    /// The degree of the generated polynomial.
    threshold: usize,
}

impl<N: NodeIdT, PK: PublicKey> SyncKeyGen<N, PK> {
    /// Creates a new `SyncKeyGen` instance, together with the `Part` message that should be
    /// multicast to all nodes.
    ///
    /// If we are not a validator but only an observer, no `Part` message is produced and no
    /// messages need to be sent.
    pub fn new<R: rand::Rng>(
        our_id: N,
        sec_key: PK::SecretKey,
        pub_keys: PubKeyMap<N, PK>,
        threshold: usize,
        rng: &mut R,
    ) -> Result<(Self, Option<Part>), Error> {
        let our_idx = pub_keys
            .keys()
            .position(|id| *id == our_id)
            .map(|idx| idx as u64);
        let key_gen = SyncKeyGen {
            our_id,
            our_idx,
            sec_key,
            pub_keys,
            parts: BTreeMap::new(),
            threshold,
        };
        if our_idx.is_none() {
            return Ok((key_gen, None)); // No part: we are an observer.
        }

        let our_part = BivarPoly::random(threshold, rng);
        let commit = our_part.commitment();
        let encrypt = |(i, pk): (usize, &PK)| {
            let row = bincode::serialize(&our_part.row(i + 1))?;
            Ok(pk.encrypt(&row, rng).map_err(Error::encrypt)?)
        };
        let rows = key_gen
            .pub_keys
            .values()
            .enumerate()
            .map(encrypt)
            .collect::<Result<Vec<Vec<u8>>, Error>>()?;
        Ok((key_gen, Some(Part(commit, rows))))
    }

    /// Returns the id of this node.
    pub fn our_id(&self) -> &N {
        &self.our_id
    }

    /// Returns the map of participating nodes and their public keys.
    pub fn public_keys(&self) -> &PubKeyMap<N, PK> {
        &self.pub_keys
    }

    /// Handles a `Part` message. If it is valid, returns an `Ack` message to be broadcast.
    ///
    /// If we are only an observer, `None` is returned instead and no messages need to be sent.
    ///
    /// All participating nodes must handle the exact same sequence of messages.
    /// Note that `handle_part` also needs to explicitly be called with this instance's own `Part`.
    pub fn handle_part<R: rand::Rng>(
        &mut self,
        sender_id: &N,
        part: Part,
        rng: &mut R,
    ) -> Result<PartOutcome, Error> {
        let sender_idx = self.node_index(sender_id).ok_or(Error::UnknownSender)?;
        let row = match self.handle_part_or_fault(sender_idx, part) {
            Ok(Some(row)) => row,
            Ok(None) => return Ok(PartOutcome::Valid(None)),
            Err(fault) => return Ok(PartOutcome::Invalid(fault)),
        };
        // The row is valid. Encrypt one value for each node and broadcast an `Ack`.
        let mut values = Vec::new();
        for (idx, pk) in self.pub_keys.values().enumerate() {
            let val = row.evaluate(idx + 1);
            let ser_val = bincode::serialize(&FieldWrap(val))?;
            values.push(pk.encrypt(ser_val, rng).map_err(Error::encrypt)?);
        }
        Ok(PartOutcome::Valid(Some(Ack(sender_idx, values))))
    }

    /// Handles an `Ack` message.
    ///
    /// All participating nodes must handle the exact same sequence of messages.
    /// Note that `handle_ack` also needs to explicitly be called with this instance's own `Ack`s.
    pub fn handle_ack(&mut self, sender_id: &N, ack: Ack) -> Result<AckOutcome, Error> {
        let sender_idx = self.node_index(sender_id).ok_or(Error::UnknownSender)?;
        Ok(match self.handle_ack_or_fault(sender_idx, ack) {
            Ok(()) => AckOutcome::Valid,
            Err(fault) => AckOutcome::Invalid(fault),
        })
    }

    /// Returns the index of the node, or `None` if it is unknown.
    fn node_index(&self, node_id: &N) -> Option<u64> {
        self.pub_keys
            .keys()
            .position(|id| id == node_id)
            .map(|idx| idx as u64)
    }

    /// Returns the number of complete parts. If this is at least `threshold + 1`, the keys can
    /// be generated, but it is possible to wait for more to increase security.
    pub fn count_complete(&self) -> usize {
        self.parts
            .values()
            .filter(|part| part.is_complete(self.threshold))
            .count()
    }

    /// Returns `true` if the part of the given node is complete.
    pub fn is_node_ready(&self, proposer_id: &N) -> bool {
        self.node_index(proposer_id)
            .and_then(|proposer_idx| self.parts.get(&proposer_idx))
            .map_or(false, |part| part.is_complete(self.threshold))
    }

    /// Returns `true` if enough parts are complete to safely generate the new key.
    pub fn is_ready(&self) -> bool {
        self.count_complete() > self.threshold
    }

    /// Returns the new secret key share and the public key set.
    ///
    /// These are only secure if `is_ready` returned `true`. Otherwise it is not guaranteed that
    /// none of the nodes knows the secret master key.
    ///
    /// If we are only an observer node, no secret key share is returned.
    ///
    /// All participating nodes must have handled the exact same sequence of `Part` and `Ack`
    /// messages before calling this method. Otherwise their key shares will not match.
    pub fn generate(&self) -> Result<(PublicKeySet, Option<SecretKeyShare>), Error> {
        let mut pk_commit = Poly::zero().commitment();
        let mut opt_sk_val = self.our_idx.map(|_| Fr::zero());
        let is_complete = |part: &&ProposalState| part.is_complete(self.threshold);
        for part in self.parts.values().filter(is_complete) {
            pk_commit += part.commit.row(0);
            if let Some(sk_val) = opt_sk_val.as_mut() {
                let row = Poly::interpolate(part.values.iter().take(self.threshold + 1));
                sk_val.add_assign(&row.evaluate(0));
            }
        }
        let opt_sk = if let Some(mut fr) = opt_sk_val {
            let sk = SecretKeyShare::from_mut(&mut fr);
            Some(sk)
        } else {
            None
        };
        Ok((pk_commit.into(), opt_sk))
    }

    /// Returns the number of nodes participating in the key generation.
    pub fn num_nodes(&self) -> usize {
        self.pub_keys.len()
    }

    /// Handles a `Part` message, or returns a `PartFault` if it is invalid.
    fn handle_part_or_fault(
        &mut self,
        sender_idx: u64,
        Part(commit, rows): Part,
    ) -> Result<Option<Poly>, PartFault> {
        if rows.len() != self.pub_keys.len() {
            return Err(PartFault::RowCount);
        }
        if let Some(state) = self.parts.get(&sender_idx) {
            if state.commit != commit {
                return Err(PartFault::MultipleParts);
            }
            return Ok(None); // We already handled this `Part` before.
        }
        // Retrieve our own row's commitment, and store the full commitment.
        let opt_idx_commit_row = self.our_idx.map(|idx| (idx, commit.row(idx + 1)));
        self.parts.insert(sender_idx, ProposalState::new(commit));
        let (our_idx, commit_row) = match opt_idx_commit_row {
            Some((idx, row)) => (idx, row),
            None => return Ok(None), // We are only an observer. Nothing to send or decrypt.
        };
        // We are a validator: Decrypt and deserialize our row and compare it to the commitment.
        let ser_row = self
            .sec_key
            .decrypt(&rows[our_idx as usize])
            .map_err(|_| PartFault::DecryptRow)?;
        let row: Poly = bincode::deserialize(&ser_row).map_err(|_| PartFault::DeserializeRow)?;
        if row.commitment() != commit_row {
            return Err(PartFault::RowCommitment);
        }
        Ok(Some(row))
    }

    /// Handles an `Ack` message, or returns an `AckFault` if it is invalid.
    fn handle_ack_or_fault(
        &mut self,
        sender_idx: u64,
        Ack(proposer_idx, values): Ack,
    ) -> Result<(), AckFault> {
        if values.len() != self.pub_keys.len() {
            return Err(AckFault::ValueCount);
        }
        let part = self
            .parts
            .get_mut(&proposer_idx)
            .ok_or(AckFault::MissingPart)?;
        if !part.acks.insert(sender_idx) {
            return Ok(()); // We already handled this `Ack` before.
        }
        let our_idx = match self.our_idx {
            Some(our_idx) => our_idx,
            None => return Ok(()), // We are only an observer. Nothing to decrypt for us.
        };
        // We are a validator: Decrypt and deserialize our value and compare it to the commitment.
        let ser_val = self
            .sec_key
            .decrypt(&values[our_idx as usize])
            .map_err(|_| AckFault::DecryptValue)?;
        let val = bincode::deserialize::<FieldWrap<Fr>>(&ser_val)
            .map_err(|_| AckFault::DeserializeValue)?
            .into_inner();
        if part.commit.evaluate(our_idx + 1, sender_idx + 1) != G1Affine::one().mul(val) {
            return Err(AckFault::ValueCommitment);
        }
        part.values.insert(sender_idx + 1, val);
        Ok(())
    }
}

/// An error in an `Ack` message sent by a faulty node.
#[derive(Clone, Copy, Eq, PartialEq, Debug, Fail)]
pub enum AckFault {
    /// The number of values differs from the number of nodes.
    #[fail(display = "The number of values differs from the number of nodes")]
    ValueCount,
    /// No corresponding Part received.
    #[fail(display = "No corresponding Part received")]
    MissingPart,
    /// Value decryption failed.
    #[fail(display = "Value decryption failed")]
    DecryptValue,
    /// Value deserialization failed.
    #[fail(display = "Value deserialization failed")]
    DeserializeValue,
    /// Value doesn't match the commitment.
    #[fail(display = "Value doesn't match the commitment")]
    ValueCommitment,
}

/// An error in a `Part` message sent by a faulty node.
#[derive(Clone, Copy, Eq, PartialEq, Debug, Fail)]
pub enum PartFault {
    /// The number of rows differs from the number of nodes.
    #[fail(display = "The number of rows differs from the number of nodes")]
    RowCount,
    /// Received multiple different Part messages from the same sender.
    #[fail(display = "Received multiple different Part messages from the same sender")]
    MultipleParts,
    /// Could not decrypt our row in the Part message.
    #[fail(display = "Could not decrypt our row in the Part message")]
    DecryptRow,
    /// Could not deserialize our row in the Part message.
    #[fail(display = "Could not deserialize our row in the Part message")]
    DeserializeRow,
    /// Row does not match the commitment.
    #[fail(display = "Row does not match the commitment")]
    RowCommitment,
}