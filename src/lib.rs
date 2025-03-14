use futures::StreamExt;
use gtfs_structures::Gtfs;
use reqwest::Client;
use serde::Deserialize;
use serde::Serialize;
use std::error::Error;

use chrono::{DateTime, Local, LocalResult, Offset, TimeZone, Utc};
use chrono_tz::Tz;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Parcour {
    #[serde(rename = "noParcours")]
    no_parcours: String,
    #[serde(rename = "codeDirectionPrincipale")]
    code_direction_principale: String,
    #[serde(rename = "codeDirectionRetour")]
    code_direction_retour: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PositionDeBus {
    #[serde(rename = "idAutobus")]
    id_autobus: i64,
    latitude: f64,
    longitude: f64,
    #[serde(rename = "idVoyage")]
    id_voyage: String,
    #[serde(rename = "dateMiseJour")]
    date_mise_jour: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ArretDansPointTemporel {
    #[serde(rename = "noArret")]
    no_arret: String,
    nom: String,
    description: String,
    latitude: f64,
    longitude: f64,
    accessible: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PointTemporelDansVoyage {
    horaire: String,
    #[serde(rename = "horaireMinutes")]
    horaire_minutes: i64,
    ntr: bool,
    arret: ArretDansPointTemporel,
}

/*#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BorneVirtuelleArretParcours {
    pub parcours: ParcourPourBorneVirtuelle,
    pub arret: ArretInfoBorneVirtuelle,
    #[serde(rename = "arretNonDesservi")]
    pub arret_non_desservi: bool,
    #[serde(rename = "descenteSeulement")]
    pub descente_seulement: bool,
    #[serde(rename = "typeParcours")]
    pub type_parcours: String,
    pub horaires: Vec<HeureBorneVirtuelle>,
}*/

pub async fn obtenir_la_liste_des_itinéraires(
    client: Client,
) -> Result<Vec<Parcour>, Box<dyn Error + Send + Sync>> {
    let parcours_url =
        "https://wsmobile.rtcquebec.ca/api/v2/horaire/ListeParcours?source=appmobileios";

    let response = client
        .get(parcours_url)
        .header(
            "User-Agent",
            "Mozilla/5.0 (Android 4.4; Tablet; rv:41.0) Gecko/41.0 Firefox/41.0",
        )
        .send()
        .await?;

    let parcours_texte = response.text().await?;

    let parcours: Vec<Parcour> = serde_json::from_str(&parcours_texte)?;

    Ok(parcours)
}

pub async fn obtenir_liste_horaire_de_autobus(
    id_voyage: &str,
    id_autobus: i64,
    client: Client,
) -> Result<Vec<PointTemporelDansVoyage>, Box<dyn Error + Send + Sync>> {
    let url = format!(
        "https://wsmobile.rtcquebec.ca/api/v3/horaire/ListeHoraire_Autobus?source=appmobileios&idVoyage={}&idAutobus={}",
        id_voyage, id_autobus
    );

    let response = client
        .get(&url)
        .header(
            "User-Agent",
            "Mozilla/5.0 (Android 4.4; Tablet; rv:41.0) Gecko/41.0 Firefox/41.0",
        )
        .send()
        .await?;
    let horaires_texte = response.text().await?;

    if horaires_texte == "[]" {
        let url = format!(
            "https://wsmobile.rtcquebec.ca/api/v3/horaire/ListeHoraire_Autobus?source=appmobileios&idVoyage={}&idAutobus={}\"",
            id_voyage, id_autobus
        );

        let response = client
            .get(&url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Android 4.4; Tablet; rv:41.0) Gecko/41.0 Firefox/41.0",
            )
            .send()
            .await?;
        let horaires_texte = response.text().await?;

        //if still empty, try again

        if horaires_texte == "[]" {
            let response = client
                .get(&url)
                .header(
                    "User-Agent",
                    "Mozilla/5.0 (Android 4.4; Tablet; rv:41.0) Gecko/41.0 Firefox/41.0",
                )
                .send()
                .await?;
            let horaires_texte = response.text().await?;

            let horaires: Vec<PointTemporelDansVoyage> = serde_json::from_str(&horaires_texte)?;
            Ok(horaires)
        } else {
            let horaires: Vec<PointTemporelDansVoyage> = serde_json::from_str(&horaires_texte)?;
            Ok(horaires)
        }
    } else {
        let horaires: Vec<PointTemporelDansVoyage> = serde_json::from_str(&horaires_texte)?;
        Ok(horaires)
    }
}

pub async fn positions(
    route: &str,
    direction: &str,
    client: Client,
) -> Result<Vec<PositionDeBus>, Box<dyn Error + Send + Sync>> {
    let url = format!(
        "https://wssiteweb.rtcquebec.ca/api/v2/horaire/ListeAutobus_Parcours/?noParcours={route}&codeDirection={direction}"
    );

    let response = client
        .get(&url)
        .header(
            "User-Agent",
            "Mozilla/5.0 (Android 4.4; Tablet; rv:41.0) Gecko/41.0 Firefox/41.0",
        )
        .send()
        .await?;

    let positions_texte = response.text().await?;

    let positions: Vec<PositionDeBus> = serde_json::from_str(&positions_texte)?;

    Ok(positions)
}

#[derive(Clone, Debug)]
pub struct ResponseGtfsRt {
    pub vehicles: Option<gtfs_realtime::FeedMessage>,
    pub voyages: Option<gtfs_realtime::FeedMessage>,
    pub alertes: Option<gtfs_realtime::FeedMessage>,
}

pub async fn faire_les_donnees_gtfs_rt(
    gtfs: &Gtfs,
    client: Client,
) -> Result<ResponseGtfsRt, Box<dyn Error + Send + Sync>> {
    let start_timer = std::time::Instant::now();

    let mut route_id_to_trips: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();

    for (trip_id, trip) in gtfs.trips.iter() {
        if trip.service_id.contains("multint") {
            continue;
        }

        if let Some(trips) = route_id_to_trips.get_mut(&trip.route_id) {
            trips.push(trip_id.clone());
        } else {
            route_id_to_trips.insert(trip.route_id.clone(), vec![trip_id.clone()]);
        }
    }

    let end_timer = start_timer.elapsed();

    println!("route_id_to_trips: {:?}", end_timer);

    let mut parcours: Option<_> = None;
    let mut tries = 0;

    while tries < 10 {
        let parcours_t = obtenir_la_liste_des_itinéraires(client.clone()).await;

        let is_success = parcours_t.is_ok();

        parcours = Some(parcours_t);

        if is_success {
            break;
        }

        tries += 1;
    }

    let parcours = parcours.unwrap();
    let parcours = parcours?;

    let mut pos_requests = vec![];

    for parcour in parcours.iter() {
        let route_id = &parcour.no_parcours;
        let direction_principale = &parcour.code_direction_principale;
        let direction_retour = &parcour.code_direction_retour;

        for direction in [direction_principale, direction_retour] {
            pos_requests.push({
                let client = client.clone();
                let route_id = route_id.clone();
                let direction = direction.clone();

                async move {
                    let pos_req = positions(route_id.as_str(), direction.as_str(), client).await;

                    match pos_req {
                        Ok(positions) => Ok((route_id, direction, positions)),
                        Err(e) => Err(e),
                    }
                }
            });
        }
    }

    let time_pos_requests = std::time::Instant::now();
    let pos_requests_buffered = futures::stream::iter(pos_requests)
        .buffer_unordered(64)
        .collect::<Vec<Result<_, _>>>()
        .await;
    println!(
        "temps écoulé pour demander des positions: {:?}",
        time_pos_requests.elapsed()
    );

    //parcours id, direction, positions
    let pos_requests_buffered = pos_requests_buffered
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

    let vec_voyage_et_autobus = pos_requests_buffered
        .iter()
        .map(|(_, _, positions)| {
            positions
                .iter()
                .map(|position| (position.id_voyage.clone(), position.id_autobus.clone()))
        })
        .flatten()
        .collect::<Vec<_>>();

    let mut horaires_requests = vec![];

    for (id_voyage, id_autobus) in vec_voyage_et_autobus.iter() {
        let client = client.clone();
        let id_voyage = id_voyage.clone();
        let id_autobus = id_autobus.clone();

        horaires_requests.push(async move {
            let mut horaires_req = None;
            let mut tries = 0;

            while tries < 5 {
                horaires_req = Some(
                    obtenir_liste_horaire_de_autobus(
                        id_voyage.as_str(),
                        id_autobus,
                        client.clone(),
                    )
                    .await,
                );

                if horaires_req.as_ref().unwrap().is_ok() {
                    break;
                }

                tries += 1;
            }

            let horaires_req = horaires_req.unwrap();

            match horaires_req {
                Ok(horaires) => Ok((id_voyage.clone(), id_autobus, horaires)),
                Err(e) => Err(e),
            }
        });
    }

    let time_horaires_requests = std::time::Instant::now();

    let horaires_requests_buffered = futures::stream::iter(horaires_requests)
        .buffer_unordered(64)
        .collect::<Vec<Result<_, _>>>()
        .await;

    for h in horaires_requests_buffered.iter() {
        if let Err(e) = h {
            eprintln!("échec de la récupération des données horaires: {:?}", e);
        }
    }

    let horaires = horaires_requests_buffered
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

    let mut horaires_hashtable = std::collections::HashMap::new();

    for (id_voyage, id_autobus, horaires) in horaires {
        horaires_hashtable.insert((id_voyage, id_autobus), horaires);
    }

    println!(
        "il est temps de récupérer les heures: {:?}",
        time_horaires_requests.elapsed()
    );

    //compute gtfs now

    let mut gtfs_vehicles = gtfs_realtime::FeedMessage {
        header: gtfs_realtime::FeedHeader {
            gtfs_realtime_version: "2.0".to_string(),
            incrementality: None,
            timestamp: Some(Utc::now().timestamp() as u64),
        },
        entity: vec![],
    };

    let mut gtfs_trips = gtfs_realtime::FeedMessage {
        header: gtfs_realtime::FeedHeader {
            gtfs_realtime_version: "2.0".to_string(),
            incrementality: None,
            timestamp: Some(Utc::now().timestamp() as u64),
        },
        entity: vec![],
    };

    for (parcours_id, direction, positions) in pos_requests_buffered {
        let gtfs_parcours_id = format!("1-{}", parcours_id);

        let voyages_possibles_en_gtfs = route_id_to_trips.get(gtfs_parcours_id.as_str());

        let mut trip_id_to_start_and_end_time: Vec<(
            String,
            chrono::DateTime<chrono_tz::Tz>,
            chrono::DateTime<chrono_tz::Tz>,
        )> = vec![];

        if let Some(voyages_possibles_en_gtfs) = voyages_possibles_en_gtfs {
            for trip_id in voyages_possibles_en_gtfs {
                let gtfs_trip = gtfs.trips.get(trip_id).unwrap();

                let service_id = &gtfs_trip.service_id;

                let service_date = gtfs.calendar_dates.get(service_id);

                if let Some(service_date) = service_date {
                    if let Some(calendar_first) = service_date.get(0) {
                        let calendar_first = calendar_first.date;
                        let reference_midnight = calc_reference_midnight(calendar_first);

                        //trip start time

                        let trip_start_time_relative =
                            gtfs_trip.stop_times[0].departure_time.unwrap();
                        let trip_start_time = reference_midnight
                            + std::time::Duration::from_secs(trip_start_time_relative as u64);

                        let trip_end_time = reference_midnight
                            + std::time::Duration::from_secs(
                                gtfs_trip.stop_times.last().unwrap().departure_time.unwrap() as u64,
                            );

                        trip_id_to_start_and_end_time.push((
                            trip_id.clone(),
                            trip_start_time,
                            trip_end_time,
                        ));
                    }
                }
            }

            trip_id_to_start_and_end_time.sort_by_key(|(_, start_time, _)| *start_time);

            let trip_id_to_start_and_end_time = trip_id_to_start_and_end_time;

            for position in positions.iter().filter(|x| x.id_voyage != "0") {
                let id_voyage = &position.id_voyage;
                let id_autobus = position.id_autobus;

                let mut trip_descriptor: Option<gtfs_realtime::TripDescriptor> = None;

                let mut gtfs_rt_horaire_list: Option<
                    Vec<gtfs_realtime::trip_update::StopTimeUpdate>,
                > = None;

                let horaires = horaires_hashtable.get(&(id_voyage.clone(), id_autobus));

                if let Some(horaires) = horaires {
                    if horaires.len() >= 1 {
                        //println!("{} voyage: {}, no autobus {}", parcours_id, id_voyage, id_autobus);

                        gtfs_rt_horaire_list = Some(
                            horaires
                                .iter()
                                .filter(|horaire| horaire.horaire_minutes != -1)
                                .map(|horaire| {
                                    let stop_id = format!("1-{}", horaire.arret.no_arret);
                                    let arrival_time = None;
                                    let departure_time =
                                        chrono::DateTime::parse_from_rfc3339(&horaire.horaire)
                                            .unwrap();

                                    let departure =
                                        Some(gtfs_realtime::trip_update::StopTimeEvent {
                                            delay: None,
                                            time: Some(departure_time.timestamp()),
                                            uncertainty: None,
                                        });

                                    gtfs_realtime::trip_update::StopTimeUpdate {
                                        stop_sequence: None,
                                        stop_id: Some(stop_id),
                                        arrival: arrival_time,
                                        departure: departure,
                                        departure_occupancy_status: None,
                                        schedule_relationship: None,
                                        stop_time_properties: None,
                                    }
                                })
                                .collect::<Vec<_>>(),
                        );

                        let current_date_montreal = chrono::Utc::now()
                            .with_timezone(&chrono_tz::America::Montreal)
                            .naive_local();

                        let voyages_gtfs_possibles_en_gtfs_pour_cette_voyage_rtc =
                            voyages_possibles_en_gtfs
                                .iter()
                                .filter(|trip_id| {
                                    let split = trip_id.split('_').collect::<Vec<_>>();

                                    if split.len() != 2 {
                                        return false;
                                    }

                                    let naive_date = split[1].replace("daily", "");

                                    if naive_date.is_empty() {
                                        return false;
                                    }

                                    if let Ok(date) =
                                        chrono::NaiveDate::parse_from_str(&naive_date, "%Y%m%d")
                                    {
                                        //check if date is not more than 1 day from now
                                        let diff = current_date_montreal
                                            .date()
                                            .signed_duration_since(date);
                                        diff.num_days().abs() < 2
                                    } else {
                                        false
                                    }
                                })
                                .filter(|trip_id| {
                                    let trip = gtfs.trips.get(*trip_id);

                                    let last_stop_gtfs =
                                        trip.unwrap().stop_times.last().unwrap().stop.clone();

                                    format!("1-{}", horaires.last().unwrap().arret.no_arret)
                                        == last_stop_gtfs.id
                                })
                                .map(|x| x.to_string())
                                .collect::<Vec<String>>();

                        //println!("{:?} pour {}, no de bus: {}", voyages_gtfs_possibles_en_gtfs_pour_cette_voyage_rtc, parcours_id, position.id_autobus);

                        let bien_horaires = horaires
                            .iter()
                            .filter(|x| x.horaire_minutes != -1)
                            .collect::<Vec<_>>();

                        let trip_id_to_start_and_end_time_for_this_voyage = match bien_horaires
                            .is_empty()
                        {
                            true => vec![],
                            false => trip_id_to_start_and_end_time
                                .iter()
                                .filter(|(trip_id, _, _)| {
                                    voyages_gtfs_possibles_en_gtfs_pour_cette_voyage_rtc
                                        .contains(trip_id)
                                })
                                .filter(|(_, scheduled_start_time, _)| {
                                    let iso_time = chrono::DateTime::parse_from_rfc3339(
                                        bien_horaires[0].horaire.as_str(),
                                    )
                                    .unwrap();

                                    let diff = scheduled_start_time.signed_duration_since(iso_time);

                                    diff.num_seconds().abs() < 60 * 60 * 3
                                })
                                .collect::<Vec<_>>(),
                        };

                        let mut diffs = trip_id_to_start_and_end_time_for_this_voyage
                            .iter()
                            .map(|(trip_id, start_time, end_time)| {
                                let iso_time = chrono::DateTime::parse_from_rfc3339(
                                    bien_horaires[bien_horaires.len() - 1].horaire.as_str(),
                                )
                                .unwrap();

                                let diff = end_time.signed_duration_since(iso_time);

                                (trip_id, diff)
                            })
                            .collect::<Vec<_>>();

                        //sort by abs of time delta
                        diffs.sort_by_key(|(_, diff)| diff.num_seconds().abs());

                        if diffs.len() >= 1 {
                            if diffs[0].1.num_seconds().abs() > 60 * 60 * 2 {
                                println!(
                                    "parcours: {} voyage: {}, autobus: {}, heure de départ de l'autobus: {}",
                                    parcours_id, id_voyage, id_autobus, diffs[0].1
                                );
                            }

                            let starting_time = trip_id_to_start_and_end_time
                                .iter()
                                .find(|(trip_id, _, _)| trip_id == diffs[0].0)
                                .unwrap()
                                .1;

                            let starting_time =
                                starting_time.with_timezone(&chrono_tz::America::Montreal);

                            /*if (starting_time.signed_duration_since(chrono::Utc::now())).num_seconds().abs() > 60 * 60 * 2 {

                            println!(
                                "
                            trip_id_to_start_time_for_this_voyage: {:?}",
                                trip_id_to_start_time_for_this_voyage
                            );

                                    println!(
                                        "parcours: {} voyage: {}, autobus: {}, starting_time: {}",
                                        parcours_id, id_voyage, id_autobus, starting_time
                                    );
                                }*/

                            trip_descriptor = Some(gtfs_realtime::TripDescriptor {
                                trip_id: Some(diffs[0].0.clone()),
                                route_id: Some(gtfs_parcours_id.clone()),
                                direction_id: None,
                                start_time: None,
                                schedule_relationship: None,
                                modified_trip: None,
                                start_date: None,
                            });
                        } else {
                            eprintln!(
                                "trip_id_to_start_time_for_this_voyage est vide!!!! parcours: {} voyage: {}, autobus: {}, {:#?}",
                                parcours_id,
                                id_voyage,
                                id_autobus,
                                trip_id_to_start_and_end_time_for_this_voyage
                            );

                            trip_descriptor = Some(gtfs_realtime::TripDescriptor {
                                trip_id: None,
                                route_id: Some(gtfs_parcours_id.clone()),
                                direction_id: None,
                                start_time: None,
                                schedule_relationship: None,
                                modified_trip: None,
                                start_date: None,
                            });
                        }
                    }
                } else {
                    println!(
                        "No stop times in rtc quebec! parcours: {} voyage: {}, no autobus {}",
                        parcours_id, id_voyage, id_autobus
                    );
                }

                // make vehicle position

                let current_vehicle_updated_time = &position.date_mise_jour;
                let split_d_t = current_vehicle_updated_time.split('T').collect::<Vec<_>>();

                let split_date = split_d_t[0].split('-').collect::<Vec<_>>();
                let split_time = split_d_t[1].split(':').collect::<Vec<_>>();

                let y = split_date[0].parse::<i32>().unwrap();
                let m = split_date[1].parse::<u32>().unwrap();
                let d = split_date[2].parse::<u32>().unwrap();

                let h = split_time[0].parse::<u32>().unwrap();
                let min = split_time[1].parse::<u32>().unwrap();
                let s = split_time[2].parse::<u32>().unwrap();

                let chrono_time = chrono::NaiveDateTime::new(
                    chrono::NaiveDate::from_ymd_opt(y, m, d).unwrap(),
                    chrono::NaiveTime::from_hms_opt(h, min, s).unwrap(),
                )
                .and_local_timezone(chrono_tz::America::Montreal);

                let find_nearest_chrono_time =
                    get_nearest_tz_from_local_result(chrono_time).unwrap();

                let vehicle = Some(gtfs_realtime::VehicleDescriptor {
                    id: Some(id_autobus.to_string()),
                    label: Some(id_autobus.to_string()),
                    license_plate: None,
                    wheelchair_accessible: None,
                });

                let v = gtfs_realtime::VehiclePosition {
                    trip: trip_descriptor.clone(),
                    vehicle: vehicle.clone(),
                    position: Some(gtfs_realtime::Position {
                        latitude: position.latitude as f32,
                        longitude: position.longitude as f32,
                        bearing: None,
                        odometer: None,
                        speed: None,
                    }),
                    current_stop_sequence: None,
                    stop_id: None,
                    current_status: None,
                    timestamp: Some(find_nearest_chrono_time.timestamp() as u64),
                    congestion_level: None,
                    occupancy_status: None,
                    occupancy_percentage: None,
                    multi_carriage_details: vec![],
                };

                let id = format!("{}-{}", parcours_id, id_autobus);

                if let Some(trip_descriptor) = trip_descriptor {
                    if let Some(gtfs_rt_horaire_list) = gtfs_rt_horaire_list {
                        let t = gtfs_realtime::TripUpdate {
                            trip: trip_descriptor,
                            vehicle: vehicle,
                            stop_time_update: gtfs_rt_horaire_list,
                            timestamp: None,
                            delay: None,
                            trip_properties: None,
                        };

                        gtfs_trips.entity.push(gtfs_realtime::FeedEntity {
                            id: id.clone(),
                            is_deleted: None,
                            trip_update: Some(t),
                            vehicle: None,
                            alert: None,
                            stop: None,
                            shape: None,
                            trip_modifications: None,
                        });
                    }
                }

                gtfs_vehicles.entity.push(gtfs_realtime::FeedEntity {
                    id: id.clone(),
                    is_deleted: None,
                    trip_update: None,
                    vehicle: Some(v),
                    alert: None,
                    stop: None,
                    shape: None,
                    trip_modifications: None,
                });
            }
        }
    }

    Ok(ResponseGtfsRt {
        vehicles: Some(gtfs_vehicles),
        voyages: Some(gtfs_trips),
        alertes: None,
    })
}

fn get_nearest_tz_from_local_result(
    local_result: LocalResult<chrono::DateTime<chrono_tz::Tz>>,
) -> Option<chrono::DateTime<chrono_tz::Tz>> {
    match local_result {
        LocalResult::Single(tz) => Some(tz),
        LocalResult::Ambiguous(tz1, tz2) => {
            let time = Utc::now();

            let offset1_sec = tz1.signed_duration_since(&time);
            let offset2_sec = tz2.signed_duration_since(&time);

            let diff1 = (offset1_sec).abs();
            let diff2 = (offset2_sec).abs();

            if diff1 <= diff2 { Some(tz1) } else { Some(tz2) }
        }
        LocalResult::None => None,
    }
}

fn calc_reference_midnight(x: chrono::NaiveDate) -> chrono::DateTime<chrono_tz::Tz> {
    let midday = x
        .and_hms_opt(12, 0, 0)
        .unwrap()
        .and_local_timezone(chrono_tz::America::Montreal)
        .unwrap();

    //subtract 12 hours
    midday - std::time::Duration::from_secs(43200)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[tokio::test]
    async fn tour_de_test_complet() {
        let client = Client::new();
        //download file https://github.com/catenarytransit/rtc_quebec_proxy_zip/raw/refs/heads/main/rtcquebec_latest.zip
        //and place it in the root of the project

        let url = "https://github.com/catenarytransit/rtc_quebec_proxy_zip/raw/refs/heads/main/rtcquebec_latest.zip";

        let telechargement = std::time::Instant::now();
        let mut response = client.get(url).send().await.unwrap();

        println!(
            "Téléchargement terminé, temps écoulé pour télécharger: {:?}",
            telechargement.elapsed()
        );

        let mut file = std::fs::File::create("./rtcquebec_latest.zip").unwrap();

        while let Some(chunk) = response.chunk().await.unwrap() {
            file.write_all(&chunk).unwrap();
        }

        let mut gtfs = gtfs_structures::GtfsReader::default()
            .read_shapes(false) // Won’t read shapes to save time and memory
            .read_from_path("./rtcquebec_latest.zip")
            .unwrap();

        gtfs.trips
            .retain(|_, trip| !trip.service_id.contains("multint"));

        println!("analyse terminée, dans {:?}", gtfs.read_duration);

        for (trip_id, trip) in gtfs.trips.iter() {
            if let Some(d) = trip.stop_times[0].departure_time {
                if d > 86400 {
                    //  println!("{}: {}", trip_id, d);
                }
            }
        }

        let faire_les_donnees_gtfs_rt = faire_les_donnees_gtfs_rt(&gtfs, client.clone())
            .await
            .unwrap();

        let mut error_date_count: usize = 0;

        if let Some(voyages) = &faire_les_donnees_gtfs_rt.voyages {
            for voyage in &voyages.entity {
                if let Some(trip_update) = &voyage.trip_update {
                    if let Some(trip_id) = &trip_update.trip.trip_id {
                        //println!("{}", trip_id);

                        // récupérer le voyage à partir du fichier GTFS
                        //get trip from gtfs file

                        let trip = gtfs.trips.get(trip_id).unwrap();

                        // obtenir la date de début du voyage
                        // get start date from trip

                        let service_id = &trip.service_id;
                        let service_date = gtfs
                            .calendar_dates
                            .get(service_id)
                            .unwrap()
                            .get(0)
                            .unwrap()
                            .date;

                        let reference_midnight = calc_reference_midnight(service_date);

                        // obtenir l'heure de début du voyage
                        // get start time from trip

                        let trip_start_time_relative = trip.stop_times[0].departure_time.unwrap();
                        let trip_start_time = reference_midnight
                            + std::time::Duration::from_secs(trip_start_time_relative as u64);

                        let trip_start_time =
                            trip_start_time.with_timezone(&chrono_tz::America::Montreal);

                        let time_from_now = trip_start_time.signed_duration_since(Utc::now());

                        if time_from_now.num_seconds() > 60 * 60 * 2 {
                            eprintln!(
                                "voyage {}, sur route {},  dans {}s",
                                trip_id,
                                trip.route_id,
                                time_from_now.num_seconds()
                            );
                            eprintln!(
                                "L'heure de début prévue est supérieure à 2 heures par rapport aux heures réelles"
                            );

                            error_date_count += 1;
                        }
                    }
                }
            }
        }

        println!("nombre d'erreurs de date: {}", error_date_count);

        //println!("{:#?}", faire_les_donnees_gtfs_rt);

        assert!(faire_les_donnees_gtfs_rt.vehicles.is_some());
        assert!(faire_les_donnees_gtfs_rt.voyages.is_some());

        //   assert!(faire_les_donnees_gtfs_rt.vehicles.unwrap().entity.len() > 0);
        //  assert!(faire_les_donnees_gtfs_rt.voyages.unwrap().entity.len() > 0);
    }
}
