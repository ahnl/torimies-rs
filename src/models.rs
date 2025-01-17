#[derive(Queryable, Clone, Debug)]
pub struct Vahti {
    pub id: i32,
    pub url: String,
    pub user_id: i64,
    pub last_updated: i64,
}

use crate::schema::Vahdit;

#[derive(Insertable)]
#[table_name = "Vahdit"]
pub struct NewVahti {
    pub url: String,
    pub user_id: i64,
    pub last_updated: i64,
}

#[derive(Queryable, Clone, Debug)]
pub struct Blacklist {
    pub id: i64,
    pub user_id: i64,
    pub seller_id: i32,
}

use crate::schema::Blacklists;

#[derive(Insertable)]
#[table_name = "Blacklists"]
pub struct NewBlacklist {
    pub user_id: i64,
    pub seller_id: i32,
}
