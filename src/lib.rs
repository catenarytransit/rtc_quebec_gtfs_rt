use gtfs_structures::Gtfs;

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn full_test_round() {
        let gtfs = Gtfs::from_url_async("https://cdn.rtcquebec.ca/Site_Internet/DonneesOuvertes/googletransit.zip")
            .await
            .unwrap();



    }
}
