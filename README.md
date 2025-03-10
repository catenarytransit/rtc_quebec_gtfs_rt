# Conversion gtfs temps réel RTC Québec

Conversion RTC Quebec GTFS rt

Cette bibliothèque logicielle Rust récupère plusieurs URL d'API utilisées par l'application RTC Nomade et les fait correspondre avec l'identifiant de voyage GTFS le plus proche possible, et produit un flux GTFS en temps réel valide.
Les contributions sont les bienvenues !

This Rust library scrapes several API urls used by the RTC Nomade app and matches it with the closest possible GTFS trip id, and produces a valid GTFS Realtime feed.
Contributions are welcome!

## Testing

Telecharge:

wget "https://cdn.rtcquebec.ca/Site_Internet/DonneesOuvertes/googletransit.zip"