// use crate::Session;
// use rusqlite::params;
// use rusqlite::Connection;
// use serde_rusqlite::{from_rows, to_params_named_with_fields};

// pub struct SqliteConn {
//     pub conn: Connection,
// }

// // shortcut Result
// type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

// impl SqliteConn {
//     pub fn new(db_uri: &str) -> Result<SqliteConn> {
//         let conn = Connection::open(db_uri)?;
//         let cipher_version: String =
//             conn.pragma_query_value(None, "cipher_version", |row| row.get(0))?;
//         println!("Sqlcipher version: {}", cipher_version);

//         Ok(SqliteConn { conn })
//     }

//     pub fn new_memory() -> Result<SqliteConn> {
//         let conn = Connection::open_in_memory()?;
//         let cipher_version: String =
//             conn.pragma_query_value(None, "cipher_version", |row| row.get(0))?;
//         println!("Sqlcipher version: {}", cipher_version);

//         Ok(SqliteConn { conn })
//     }

//     pub fn create_table(&self) -> Result<()> {
//         self.conn.execute_batch(
//             "
//                 BEGIN;
//                 CREATE TABLE IF NOT EXISTS `sessions` (
//                     `id` uuid NOT NULL, 
//                     `sk` text NOT NULL DEFAULT '',
//                     `author` text NOT NULL DEFAULT '',
//                     `description` text NOT NULL DEFAULT '',
//                     `repo` text NOT NULL DEFAULT '',
//                     PRIMARY KEY (`id`));

                
//                 COMMIT;
//             ",
//         )?;

//         Ok(())
//     }
// }

// pub struct ProviderSvc {
//     db: SqliteConn,
// }

// impl ProviderSvc {
//     pub fn all_manual_item(&self) -> Result<Vec<Session>> {
//         let mut stmt = self.db.conn.prepare("SELECT * FROM manual_items")?;
//         let items = from_rows::<Session>(stmt.query([])?)
//             .into_iter()
//             .map(|i| i.unwrap())
//             .collect();

//         Ok(items)
//     }

//     pub fn create_manual_item(&self, mi: &Session) -> Result<()> {
//         self.db.conn.execute(
//             "INSERT INTO manual_items (
//                 id,
//                 provider_name,
//                 category,
//                 item_table_id,
//                 type,
//                 description,
//                 value
//         ) VALUES 
//         (:id, :provider_name, :category, :item_table_id, :type, :description, :value)",
//             to_params_named_with_fields(
//                 mi,
//                 &[
//                     "id",
//                     "provider_name",
//                     "category",
//                     "item_table_id",
//                     "type",
//                     "description",
//                     "value",
//                 ],
//             )?
//             .to_slice()
//             .as_slice(),
//         )?;
        
//         Ok(())
//     }

//     pub fn update_manual_item(&self, mi: &Session) -> Result<()> {
//         self.db.conn.execute(
//             "UPDATE manual_items SET 
//                 provider_name = :provider_name,
//                 category = :category,
//                 item_table_id = :item_table_id,
//                 type = :type,
//                 description = :description,
//                 value = :value 
//             WHERE id = :id",
//             to_params_named_with_fields(
//                 mi,
//                 &[
//                     "id",
//                     "provider_name",
//                     "category",
//                     "item_table_id",
//                     "type",
//                     "description",
//                     "value",
//                 ],
//             )?
//             .to_slice()
//             .as_slice(),
//         )?;
       
//         Ok(())
//     }

//     pub fn delete_manual_item(&self, id: &str) -> Result<()> {
//         self.db
//             .conn
//             .execute("DELETE FROM manual_items WHERE id = ?1", params![id])?;
       
//         Ok(())
//     }
// }
