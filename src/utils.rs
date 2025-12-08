use base64::Engine as _;
use base64::engine::general_purpose;
use rand::RngCore;
use sqlx::PgTransaction;

pub async fn unique_string(
    txn: &mut PgTransaction<'_>,
    table: &str,
    column: &str,
    bytes: usize,
) -> String {
    let sql = format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE {column} = $1)");
    let mut buffer = vec![0u8; bytes];

    loop {
        rand::thread_rng().fill_bytes(&mut buffer);
        let candidate = general_purpose::URL_SAFE_NO_PAD.encode(&buffer);

        let exists = sqlx::query_scalar::<_, bool>(&sql)
            .bind(&candidate)
            .fetch_one(&mut **txn)
            .await
            .unwrap();

        if !exists {
            return candidate;
        }
    }
}
