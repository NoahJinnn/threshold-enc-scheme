pub mod dkg;
fn main() {
    println!("Hello, world!");
}

// test
#[cfg(test)]
mod test {
    use std::collections::BTreeMap;
    use std::sync::Arc;

    use super::dkg::{to_pub_keys, AckOutcome, PartOutcome, PubKeyMap, SyncKeyGen};
    use threshold_crypto::{SecretKey, SignatureShare};

    #[test]
    fn test_all() {
        // Use the OS random number generator for any randomness:
        let mut rng = rand::rngs::OsRng::new().expect("Could not open OS random number generator.");

        // Two out of four shares will suffice to sign or encrypt something.
        let (threshold, node_num) = (1, 4);

        // Generate individual key pairs for encryption. These are not suitable for threshold schemes.
        let sec_keys: Vec<SecretKey> = (0..node_num).map(|_| rand::random()).collect();
        let pub_keys = to_pub_keys(sec_keys.iter().enumerate());

        // Create the `SyncKeyGen` instances. The constructor also outputs the part that needs to
        // be sent to all other participants, so we save the parts together with their sender ID.
        let mut nodes = BTreeMap::new();
        let mut parts = Vec::new();
        for (id, sk) in sec_keys.into_iter().enumerate() {
            let (sync_key_gen, opt_part) =
                SyncKeyGen::new(id, sk, pub_keys.clone(), threshold, &mut rng).unwrap_or_else(
                    |_| panic!("Failed to create `SyncKeyGen` instance for node #{}", id),
                );
            nodes.insert(id, sync_key_gen);
            parts.push((id, opt_part.unwrap())); // Would be `None` for observer nodes.
        }

        // All nodes now handle the parts and send the resulting `Ack` messages.
        let mut acks = Vec::new();
        for (sender_id, part) in parts {
            for (&id, node) in &mut nodes {
                match node
                    .handle_part(&sender_id, part.clone(), &mut rng)
                    .expect("Failed to handle Part")
                {
                    PartOutcome::Valid(Some(ack)) => acks.push((id, ack)),
                    PartOutcome::Invalid(fault) => panic!("Invalid Part: {:?}", fault),
                    PartOutcome::Valid(None) => {
                        panic!("We are not an observer, so we should send Ack.")
                    }
                }
            }
        }

        // Finally, we handle all the `Ack`s.
        for (sender_id, ack) in acks {
            for node in nodes.values_mut() {
                match node
                    .handle_ack(&sender_id, ack.clone())
                    .expect("Failed to handle Ack")
                {
                    AckOutcome::Valid => (),
                    AckOutcome::Invalid(fault) => panic!("Invalid Ack: {:?}", fault),
                }
            }
        }

        // We have all the information and can generate the key sets.
        // Generate the public key set; which is identical for all nodes.
        let pub_key_set = nodes[&0]
            .generate()
            .expect("Failed to create `PublicKeySet` from node #0")
            .0;
        let mut secret_key_shares = BTreeMap::new();
        for (&id, node) in &mut nodes {
            assert!(node.is_ready());
            let (pks, opt_sks) = node.generate().unwrap_or_else(|_| {
                panic!(
                    "Failed to create `PublicKeySet` and `SecretKeyShare` for node #{}",
                    id
                )
            });
            assert_eq!(pks, pub_key_set); // All nodes now know the public keys and public key shares.
            let sks = opt_sks.expect("Not an observer node: We receive a secret key share.");
            secret_key_shares.insert(id, sks);
        }

        // Two out of four nodes can now sign a message. Each share can be verified individually.
        let msg = "Nodes 0 and 1 does not agree with this.";
        let mut sig_shares: BTreeMap<usize, SignatureShare> = BTreeMap::new();
        for (&id, sks) in &secret_key_shares {
            if id != 0 && id != 1 {
                let sig_share = sks.sign(msg);
                let pks = pub_key_set.public_key_share(id);
                assert!(pks.verify(&sig_share, msg));
                sig_shares.insert(id, sig_share);
            }
        }

        // Two signatures are over the threshold. They are enough to produce a signature that matches
        // the public master key.
        let sig = pub_key_set
            .combine_signatures(&sig_shares)
            .expect("The shares can be combined.");
        assert!(pub_key_set.public_key().verify(&sig, msg));
    }
}
