use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use std::env;

#[derive(Debug, Queryable)]
pub struct FinalizedTask {
    pub id: i32,
    pub task_id: String,
    pub image_id: String,
    pub public_input_hash: String,
    pub proof_hash: String,
}

#[derive(Insertable)]
#[diesel(table_name = finalized_tasks)]
pub struct NewFinalizedTask<'a> {
    pub task_id: &'a str,
    pub image_id: &'a str,
    pub public_input_hash: &'a str,
    pub proof_hash: &'a str,
}

table! {
    finalized_tasks (id) {
        id -> Integer,
        task_id -> Text,
        image_id -> Text,
        public_input_hash -> Text,
        proof_hash -> Text,
    }
}

pub fn establish_connection() -> SqliteConnection {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    SqliteConnection::establish(&database_url).expect(&format!("Error connecting to {}", database_url))
}

pub fn create_finalized_task<'a>(
    conn: &mut SqliteConnection,
    task_id: &'a str,
    image_id: &'a str,
    public_input_hash: &'a str,
    proof_hash: &'a str,
) -> usize {
    use self::finalized_tasks;

    let new_task = NewFinalizedTask {
        task_id,
        image_id,
        public_input_hash,
        proof_hash,
    };

    diesel::insert_into(finalized_tasks::table)
        .values(&new_task)
        .execute(conn)
        .expect("Error saving new finalized task")
}
