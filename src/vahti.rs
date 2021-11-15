use crate::tori::parse::*;
use crate::Database;
use crate::ItemHistory;
use crate::Mutex;
use chrono::{Local, TimeZone};
use serenity::{client::Context, http::Http};
use std::sync::Arc;

#[derive(Clone)]
pub struct Vahti {
    pub url: String,
    pub user_id: i64,
    pub last_updated: i64,
}

pub async fn new_vahti(ctx: &Context, url: &str, userid: u64) -> String {
    let db = ctx.data.read().await.get::<Database>().unwrap().clone();
    if db
        .fetch_vahti(url, userid.try_into().unwrap())
        .await
        .is_ok()
    {
        info!("Not adding a pre-defined Vahti {} for user {}", url, userid);
        return "Vahti on jo määritelty!".to_string();
    }
    match db.add_vahti_entry(url, userid.try_into().unwrap()).await {
        Ok(_) => "Vahti lisätty!".to_string(),
        Err(_) => "Virhe tapahtui vahdin lisäyksessä!".to_string(),
    }
}

pub async fn remove_vahti(ctx: &Context, url: &str, userid: u64) -> String {
    let db = ctx.data.read().await.get::<Database>().unwrap().clone();
    if db
        .fetch_vahti(url, userid.try_into().unwrap())
        .await.is_err()
    {
        info!("Not removing a nonexistant vahti!");
        return "Kyseistä vahtia ei ole määritelyt, tarkista että kirjoiti linkin oikein".to_string();
    }
    match db.remove_vahti_entry(url, userid.try_into().unwrap()).await {
        Ok(_) => "Vahti poistettu!".to_string(),
        Err(_) => "Virhe tapahtui vahdin poistamisessa!".to_string(),
    }
}

pub async fn update_all_vahtis(
    db: Arc<Database>,
    itemhistory: &mut Arc<Mutex<ItemHistory>>,
    http: &Http,
) {
    itemhistory.lock().await.purge_old();
    let vahtis = db.fetch_all_vahtis().await.unwrap();
    update_vahtis(db, itemhistory, http, vahtis).await;
}

pub async fn update_vahtis(
    db: Arc<Database>,
    itemhistory: &mut Arc<Mutex<ItemHistory>>,
    http: &Http,
    vahtis: Vec<Vahti>,
) {
    let mut currenturl = String::new();
    let mut currentitems = Vec::new();
    let test = std::time::Instant::now();
    for vahtichunks in vahtis.chunks(5) {
        let vahtichunks = vahtichunks.iter().zip(vahtichunks.iter().map(|vahti| {
            if currenturl != vahti.url {
                currenturl = vahti.url.clone();
                let mut url = "https://api.tori.fi/api/v1.2/public/ads".to_owned()
                    + &currenturl.to_owned()[currenturl.find('?').unwrap()..];
                let mut startprice: String = "".to_string();
                let mut endprice: String = "".to_string();
                let mut price_set = false;
                if url.contains("ps=") {
                    let index = url.find("ps=").unwrap();
                    let endindex = url[index..].find('&').unwrap_or(url.len()-index);
                    startprice = url[index+3..endindex+index].to_string();
                    price_set = true;
                }
                if url.contains("pe=") {
                    let index = url.find("pe=").unwrap();
                    let endindex = url[index..].find('&').unwrap_or(url.len()-index);
                    endprice = url[index+3..endindex+index].to_string();
                    price_set = true;
                }
                url = url.replace("cg=", "category=");
                // because in the API category=0 yealds no results and in the search it just means
                // no category was specified
                url = url.replace("&category=0", "");
                url = url.replace("st=", "ad_type=");
                url = url.replace("m=", "area=");
                url = url.replace("ca=", "region=");
                if price_set {
                    url = url + &format!("&suborder={}-{}", &startprice, &endprice);
                }
                info!("Sending query: {}", url);
                let response = reqwest::get(url);
                Some(response)
            } else {
                None
            }
        }));
        for (vahti, request) in vahtichunks {
            if let Some(req) = request {
                currentitems = api_parse_after(
                    &req.await.unwrap().text().await.unwrap(),
                    vahti.last_updated,
                )
                .await;
            }
            if !currentitems.is_empty() {
                db.vahti_updated(vahti.clone(), Some(currentitems[0].published))
                    .await
                    .unwrap();
            }
            info!("Got {} items", currentitems.len());
            for item in currentitems.iter().rev() {
                if itemhistory.lock().await.contains(item.ad_id, vahti.user_id) {
                    info!("Item {} in itemhistory! Skipping!", item.ad_id);
                    continue;
                }
                itemhistory
                    .lock()
                    .await
                    .add_item(item.ad_id,vahti.user_id, chrono::Local::now().timestamp());
                let user = http
                    .get_user(vahti.user_id.try_into().unwrap())
                    .await
                    .unwrap();
                user.dm(http, |m| {
                    m.embed(|e| {
                        e.color(serenity::utils::Color::DARK_GREEN);
                        e.description(format!("[{}]({})\n[Hakulinkki]({})", item.title, item.url, vahti.url));
                        e.field("Hinta", format!("{} €", item.price), true);
                        e.field("Myyjä", item.seller_name.clone(), true);
                        e.field("Sijainti", item.location.clone(), true);
                        e.field(
                            "Ilmoitus Jätetty",
                            Local.timestamp(item.published, 0).format("%d/%m/%Y %R"),
                            true,
                        );
                        e.field("Ilmoitustyyppi", item.ad_type.to_string(), true);
                        e.image(item.img_url.clone())
                    })
                })
                .await
                .unwrap();
            }
        }
    }
    info!("Finished requests in {} ms", test.elapsed().as_millis());
}