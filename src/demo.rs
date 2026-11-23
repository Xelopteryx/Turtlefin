//! Essai d'un thème : `turtlefin --theme-demo=<fichier .tftheme>` (bouton « Essayer dans Turtlefin » du
//! Turtlefin Theme Creator).
//!
//! Le vrai Turtlefin s'ouvre sur un faux serveur Jellyfin qui tourne dans ce même programme
//! (127.0.0.1, port libre) : bibliothèques, films, séries, saisons, épisodes, collections, recherche,
//! favoris, vu / non vu et lecture (une mire animée). Rien ne sort de la machine, et ni la
//! configuration ni le cache de l'utilisateur ne sont touchés (dossiers à part, vidés à chaque essai).
//! Le thème est relu dès que son fichier change : l'essai suit le créateur en direct.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Mutex, OnceLock};

use serde_json::{json, Value};

pub const USER: &str = "Démo";
/// Assez long pour ne pas déclencher l'alerte « mot de passe trop simple » pendant l'essai.
pub const PASSWORD: &str = "Essai-du-theme-Turtlefin-2026";
const USER_ID: &str = "demo0000000000000000000000000001";
const SERVER_ID: &str = "demo0000000000000000000000000000";

/// Fichier du thème essayé (relu quand il change).
static THEME_FILE: OnceLock<std::path::PathBuf> = OnceLock::new();

/// Prépare l'essai : dossiers à part, préférences avec le thème, faux serveur. Rend son adresse.
#[cfg_attr(target_os = "android", allow(dead_code))]
pub fn start(theme_file: &str) -> Option<String> {
    let real = crate::config::ui_prefs();
    let theme = crate::theme::load(std::path::Path::new(theme_file))?;
    let root = crate::paths::cache_dir()?.join("theme-demo");
    for sub in ["config", "data", "cache"] {
        let _ = std::fs::remove_dir_all(root.join(sub));
        std::fs::create_dir_all(root.join(sub)).ok()?;
    }
    std::env::set_var("TURTLEFIN_CONFIG_DIR", root.join("config"));
    std::env::set_var("TURTLEFIN_DATA_DIR", root.join("data"));
    std::env::set_var("TURTLEFIN_CACHE_DIR", root.join("cache"));
    std::env::set_var("TURTLEFIN_NO_DISCOVERY", "1");
    let mut p = real;
    p.theme = format!("custom:{}", theme.name);
    p.themes = vec![theme];
    p.autostart_user.clear();
    p.autostart_server.clear();
    p.tutorial_offered = true;
    p.tutorial_pending = false;
    p.mods.clear();
    // Pas de choix de langue à chaque essai : celle du système si aucune n'est encore choisie.
    if p.language.is_empty() {
        p.language = crate::i18n::system_language();
    }
    // Le fond animé et la musique font partie de ce qu'on essaie.
    p.anim.scenery = true;
    p.music = true;
    crate::config::save_ui_prefs(&p);
    let _ = THEME_FILE.set(std::path::PathBuf::from(theme_file));

    serve_local()
}

/// Adresse du faux serveur, s'il tourne (session de démonstration).
pub fn is_local(server: &str) -> bool {
    local_url().is_some_and(|u| server.trim_end_matches('/') == u)
}

fn local_url() -> Option<&'static str> {
    URL.get().and_then(|u| u.as_deref())
}

static URL: OnceLock<Option<String>> = OnceLock::new();

/// Lance le faux serveur (une seule fois) et rend son adresse. Port 8097 s'il est libre : la même
/// adresse d'une fois à l'autre, le cache des images ne se remplit pas de copies ; sinon un port libre.
pub fn serve_local() -> Option<String> {
    URL.get_or_init(|| {
        let listener = TcpListener::bind(("127.0.0.1", 8097)).or_else(|_| TcpListener::bind(("127.0.0.1", 0))).ok()?;
        let port = listener.local_addr().ok()?.port();
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                std::thread::spawn(move || {
                    let _ = serve(stream);
                });
            }
        });
        Some(format!("http://127.0.0.1:{port}"))
    })
    .clone()
}

/// Suit le fichier du thème : dès qu'il change (le créateur l'a réécrit), le thème est reposé.
#[cfg_attr(target_os = "android", allow(dead_code))]
pub fn watch(ui: &crate::AppWindow) {
    use slint::ComponentHandle;
    let Some(path) = THEME_FILE.get() else { return };
    let stamp = |p: &std::path::Path| std::fs::metadata(p).and_then(|m| m.modified()).ok();
    let mut last = stamp(path);
    let weak = ui.as_weak();
    let timer = Box::leak(Box::new(slint::Timer::default()));
    timer.start(slint::TimerMode::Repeated, std::time::Duration::from_millis(500), move || {
        let now = stamp(path);
        if now == last {
            return;
        }
        last = now;
        let (Some(u), Some(t)) = (weak.upgrade(), crate::theme::load(path)) else { return };
        let mut p = crate::config::ui_prefs();
        p.theme = format!("custom:{}", t.name);
        p.themes = vec![t.clone()];
        crate::config::save_ui_prefs(&p);
        crate::theme::apply(&u, &t);
    });
}

// ---------------------------------------------------------------------------------------------
// Catalogue
// ---------------------------------------------------------------------------------------------

const PEOPLE: [&str; 10] = [
    "Maëlle Kerdoncuff", "Octave Lemarchand", "Inès Valcourt", "Basile Aubrac", "Solène Trévidic", "Ludovic Marsan", "Apolline Riquier",
    "Timothée Vasseur", "Capucine Hérault", "Anselme Dufresnoy",
];

/// Textes du catalogue : en français si l'interface l'est, en anglais sinon (les captures et les
/// essais de thème dans une autre langue n'affichent pas de titres français).
struct Words {
    genres: [&'static str; 8],
    tags: [&'static str; 8],
    studios: [&'static str; 5],
    movies: [&'static str; 30],
    shows: [&'static str; 10],
    sets: [(&'static str, [usize; 3]); 2],
    overview: [&'static str; 5],
    roles: [&'static str; 5],
    libs: [&'static str; 3],
    season: &'static str,
    episodes: [&'static str; 8],
    ratings: [&'static str; 4],
    stream: &'static str,
}

const FR: Words = Words {
    genres: ["Aventure", "Science-fiction", "Comédie", "Drame", "Animation", "Policier", "Fantastique", "Documentaire"],
    tags: ["Voyage", "Espace", "Amitié", "Mystère", "Survival Horror", "Musique", "Nature", "Robots"],
    studios: ["Studio Carapace", "Atelier Boréal", "Les Films du Récif", "Lagune Animation", "Phare Productions"],
    movies: [
    "La Baie des Tortues", "Marée Haute", "Le Dernier Phare", "Orbite Lente", "Les Cartographes", "Sable Rouge", "Nuit Polaire", "Le Jardin Suspendu",
    "Courant Froid", "Mille Lanternes", "Le Passeur", "Atlas Bleu", "Brume", "Le Grand Récif", "Signal Faible", "Les Veilleurs", "Tempête de Papier",
    "Zénith", "Le Voleur de Vent", "Eaux Calmes", "Minuit à Quai", "Kilomètre Zéro", "Les Funambules", "Comète", "Dernière Station", "Hors Saison",
    "Le Chant du Cargo", "Poussière d'Étoiles", "Rivage", "Un Été en Apnée",
],
    shows: [
    "Les Naufragés du Ciel", "Archipel", "Brigade des Marées", "Chroniques du Phare", "Dix Mille Lieues", "Écume", "Station Corail", "Le Septième Port",
    "Vents Contraires", "Abysses",
],
    sets: [("Saga des Marées", [1, 8, 19]), ("Trilogie du Phare", [2, 15, 24])],
    overview: [
    "Une traversée qui devait durer trois jours, un équipage qui ne se connaît pas, et une carte dont il manque la moitié.",
    "Quand le signal revient après vingt ans de silence, il faut bien que quelqu'un aille voir d'où il vient.",
    "Ils pensaient avoir tout prévu. La mer, elle, n'avait rien promis.",
    "Entre deux escales, une rencontre remet en question tout ce qu'elle croyait savoir de sa famille.",
    "Ceci est un média de démonstration : il n'existe que pour découvrir Turtlefin.",
],
    roles: ["La capitaine", "Le mécano", "L'inconnue", "Le gardien", "La cartographe"],
    libs: ["Films", "Séries", "Collections"],
    season: "Saison",
    episodes: ["Le départ", "Par gros temps", "Ce que disent les cartes", "L'escale", "Sous la surface", "Le signal", "Retour au port", "Dernier quart"],
    ratings: ["Tous publics", "10+", "12+", "16+"],
    stream: "Mire de démonstration",
};

const EN: Words = Words {
    genres: ["Adventure", "Science Fiction", "Comedy", "Drama", "Animation", "Crime", "Fantasy", "Documentary"],
    tags: ["Travel", "Space", "Friendship", "Mystery", "Survival Horror", "Music", "Nature", "Robots"],
    studios: ["Shell Studio", "Boreal Workshop", "Reef Pictures", "Lagoon Animation", "Lighthouse Productions"],
    movies: [
        "Turtle Bay", "High Tide", "The Last Lighthouse", "Slow Orbit", "The Mapmakers", "Red Sand", "Polar Night", "The Hanging Garden",
        "Cold Current", "A Thousand Lanterns", "The Ferryman", "Blue Atlas", "Mist", "The Great Reef", "Weak Signal", "The Watchers", "Paper Storm",
        "Zenith", "The Wind Thief", "Still Waters", "Midnight at the Dock", "Mile Zero", "The Tightrope Walkers", "Comet", "Last Station", "Off Season",
        "Song of the Freighter", "Stardust", "Shoreline", "A Summer Underwater",
    ],
    shows: [
        "Castaways of the Sky", "Archipelago", "Tide Patrol", "Lighthouse Chronicles", "Ten Thousand Leagues", "Seafoam", "Coral Station",
        "The Seventh Port", "Headwinds", "Abyss",
    ],
    sets: [("Tides Saga", [1, 8, 19]), ("Lighthouse Trilogy", [2, 15, 24])],
    overview: [
        "A crossing meant to last three days, a crew of strangers, and a map with half of it missing.",
        "When the signal returns after twenty years of silence, someone has to go and see where it comes from.",
        "They thought they had planned for everything. The sea had promised nothing.",
        "Between two ports of call, a chance meeting calls into question everything she thought she knew about her family.",
        "This is a demo title: it only exists to show you around Turtlefin.",
    ],
    roles: ["The Captain", "The Mechanic", "The Stranger", "The Keeper", "The Mapmaker"],
    libs: ["Movies", "Shows", "Collections"],
    season: "Season",
    episodes: ["Departure", "Heavy Weather", "What the Maps Say", "The Stopover", "Below the Surface", "The Signal", "Back to Port", "Last Watch"],
    ratings: ["All ages", "10+", "12+", "16+"],
    stream: "Demo test pattern",
};

/// Vrais films libres de la Blender Foundation / Blender Studio (images de Wikimedia Commons, voir
/// src/demo-assets/CREDITS.txt) : clé des images, titre, année, genres (rangs dans `Words::genres`),
/// réalisation, studio, licence.
struct Film {
    key: &'static str,
    name: &'static str,
    year: u32,
    genres: [usize; 2],
    director: &'static str,
    studio: &'static str,
    license: &'static str,
    poster: &'static [u8],
    backdrop: &'static [u8],
}

macro_rules! film {
    ($key:literal, $name:literal, $year:literal, $g:expr, $dir:literal, $studio:literal, $lic:literal) => {
        Film {
            key: $key,
            name: $name,
            year: $year,
            genres: $g,
            director: $dir,
            studio: $studio,
            license: $lic,
            poster: include_bytes!(concat!("demo-assets/", $key, "-poster.webp")),
            backdrop: include_bytes!(concat!("demo-assets/", $key, "-backdrop.webp")),
        }
    };
}

const FILMS: [Film; 8] = [
    film!("sprite", "Sprite Fright", 2021, [6, 2], "Matthew Luhn", "Blender Studio", "CC BY 4.0"),
    film!("coffee", "Coffee Run", 2020, [2, 4], "Hjalti Hjálmarsson", "Blender Studio", "CC BY 4.0"),
    film!("spring", "Spring", 2019, [6, 4], "Andy Goralczyk", "Blender Foundation", "CC BY 4.0"),
    film!("cosmos", "Cosmos Laundromat", 2015, [2, 6], "Mathieu Auvray", "Blender Foundation", "CC BY 4.0"),
    film!("tos", "Tears of Steel", 2012, [1, 0], "Ian Hubert", "Blender Foundation", "CC BY 3.0"),
    film!("sintel", "Sintel", 2010, [6, 0], "Colin Levy", "Blender Foundation", "CC BY 3.0"),
    film!("bbb", "Big Buck Bunny", 2008, [2, 4], "Sacha Goedegebure", "Blender Foundation", "CC BY 3.0"),
    film!("ed", "Elephants Dream", 2006, [1, 4], "Bassam Kurdali", "Blender Foundation", "CC BY 2.5"),
];

/// Résumés des films (même ordre que FILMS), rédigés pour la démo.
const FILM_FR: [&str; 8] = [
    "Une sortie en forêt tourne mal quand une bande d'ados croise des lutins bien moins mignons qu'ils n'en ont l'air.",
    "Portée par la caféine, une jeune femme traverse à toute allure les souvenirs de sa vie.",
    "Une bergère et son chien affrontent les esprits anciens de la montagne pour faire revenir le printemps.",
    "Sur une île désolée, un mouton las de tout rencontre un vendeur excentrique qui lui propose une vie nouvelle.",
    "Dans un Amsterdam du futur, des scientifiques tentent de sauver le monde des robots géants… en réparant un cœur brisé.",
    "Une jeune femme parcourt des terres hostiles à la recherche du petit dragon qu'elle avait recueilli.",
    "Un lapin géant et paisible voit sa journée gâchée par trois rongeurs farceurs, et décide de leur rendre la monnaie de leur pièce.",
    "Deux hommes errent dans une machine sans fin, où le plus âgé tente d'imposer sa vision du monde au plus jeune.",
];
const FILM_EN: [&str; 8] = [
    "A trip to the woods goes wrong when a group of teenagers meets forest sprites far less cute than they look.",
    "Fuelled by caffeine, a young woman races through the memories of her life.",
    "A shepherd girl and her dog face the ancient spirits of the mountain to bring back the spring.",
    "On a desolate island, a weary sheep meets an eccentric salesman who offers him a brand-new life.",
    "In a future Amsterdam, a group of scientists try to save the world from giant robots… by mending a broken heart.",
    "A young woman crosses hostile lands in search of the baby dragon she once took in.",
    "A gentle giant rabbit has his day ruined by three mischievous rodents, and decides it is time to pay them back.",
    "Two men wander through an endless machine, the elder trying to impose his view of the world on the younger.",
];

fn film_of(item: &Value) -> Option<&'static Film> {
    let id = item["Id"].as_str()?;
    FILMS.iter().find(|f| id.strip_prefix("film-") == Some(f.key))
}

fn w() -> &'static Words {
    if crate::i18n::current() == "fr" { &FR } else { &EN }
}

/// Durée de la mire lue à la place des vidéos (secondes) et sa cadence.
const VIDEO_SECS: u64 = 90;
const VIDEO_FPS: u64 = 12;
const VIDEO_W: usize = 320;
const VIDEO_H: usize = 180;

fn catalog() -> &'static Mutex<Vec<Value>> {
    static CAT: OnceLock<Mutex<Vec<Value>>> = OnceLock::new();
    CAT.get_or_init(|| Mutex::new(build()))
}

fn user_data(i: usize, playable: bool) -> Value {
    let played = playable && i % 5 == 0;
    // Vu ou commencé, pas les deux.
    let pct = if playable && !played && i % 4 == 1 { 20.0 + (i * 13 % 60) as f64 } else { 0.0 };
    json!({
        "Played": played,
        "IsFavorite": i % 6 == 2,
        "PlayedPercentage": if pct > 0.0 { json!(pct) } else { Value::Null },
        "PlaybackPositionTicks": (pct / 100.0 * VIDEO_SECS as f64 * 1e7) as i64,
    })
}

fn common(id: &str, name: &str, kind: &str, i: usize) -> Value {
    let people: Vec<Value> = (0..5)
        .map(|k| {
            let n = (i * 3 + k * 7) % PEOPLE.len();
            let role = w().roles[k];
            json!({ "Id": format!("person{n}"), "Name": PEOPLE[n], "Role": role, "Type": "Actor" })
        })
        .collect();
    let rating = w().ratings[i % 4];
    json!({
        "Id": id,
        "Name": name,
        "Type": kind,
        "Overview": w().overview[i % 5],
        "Genres": [w().genres[i % 8], w().genres[(i * 3 + 1) % 8]],
        "Tags": [w().tags[i % 8], w().tags[(i * 5 + 2) % 8]],
        "Studios": [{ "Name": w().studios[i % 5], "Id": format!("studio{}", i % 5) }],
        "People": people,
        "OfficialRating": rating,
        "CommunityRating": 5.5 + ((i * 37) % 42) as f64 / 10.0,
        "ProductionYear": 1998 + (i * 7) % 28,
        "PremiereDate": format!("{}-0{}-1{}T00:00:00.0000000Z", 1998 + (i * 7) % 28, 1 + i % 9, i % 9),
        "ImageTags": { "Primary": "demo", "Logo": "demo", "Thumb": "demo" },
        "BackdropImageTags": ["demo"],
        "DateCreated": i,
    })
}

fn playable(v: &mut Value, id: &str, i: usize) {
    v["RunTimeTicks"] = json!(VIDEO_SECS * 10_000_000);
    v["UserData"] = user_data(i, true);
    v["MediaSources"] = json!([media_source(id)]);
}

fn media_source(id: &str) -> Value {
    json!({
        "Id": id, "Protocol": "File", "Container": "y4m", "SupportsDirectPlay": true, "SupportsDirectStream": true,
        "Size": video_len(),
        "MediaStreams": [{ "Type": "Video", "Index": 0, "Codec": "rawvideo", "DisplayTitle": w().stream, "IsDefault": true }],
    })
}

fn build() -> Vec<Value> {
    let mut out = Vec::new();
    let libs = w().libs;
    for (id, name, kind) in [("libmovies", libs[0], "movies"), ("libshows", libs[1], "tvshows"), ("libsets", libs[2], "boxsets")] {
        out.push(json!({ "Id": id, "Name": name, "Type": "CollectionFolder", "CollectionType": kind, "IsFolder": true, "ImageTags": { "Primary": "demo" } }));
    }
    let fr = crate::i18n::current() == "fr";
    for (i, f) in FILMS.iter().enumerate() {
        let id = format!("film-{}", f.key);
        let mut v = common(&id, f.name, "Movie", i + 3);
        let credit = if fr {
            format!(" Film libre de la {} ({}).", f.studio, f.license)
        } else {
            format!(" Free film by the {} ({}).", f.studio, f.license)
        };
        v["Overview"] = json!(format!("{}{credit}", if fr { FILM_FR[i] } else { FILM_EN[i] }));
        v["Genres"] = json!([w().genres[f.genres[0]], w().genres[f.genres[1]]]);
        v["Studios"] = json!([{ "Name": f.studio, "Id": "studioblender" }]);
        v["People"] = json!([{ "Id": format!("dir-{}", f.key), "Name": f.director, "Role": "", "Type": "Director" }]);
        v["ProductionYear"] = json!(f.year);
        v["PremiereDate"] = json!(format!("{}-0{}-1{}T00:00:00.0000000Z", f.year, 1 + i % 9, i % 9));
        // Pas de logo : le titre s'affiche en texte (pas de faux logo dessiné pour un vrai film).
        v["ImageTags"] = json!({ "Primary": "film", "Thumb": "film" });
        // Arrivés en dernier : en tête de « Ajouts récents ».
        v["DateCreated"] = json!(1000 + FILMS.len() - i);
        v["Parent"] = json!("libmovies");
        playable(&mut v, &id, i + 3);
        out.push(v);
    }
    for (i, name) in w().movies.iter().enumerate() {
        let id = format!("movie{i:02}");
        let mut v = common(&id, name, "Movie", i);
        v["Parent"] = json!("libmovies");
        playable(&mut v, &id, i);
        out.push(v);
    }
    for (i, (name, movies)) in w().sets.iter().enumerate() {
        let mut v = common(&format!("set{i}"), name, "BoxSet", i + 40);
        v["Parent"] = json!("libsets");
        v["IsFolder"] = json!(true);
        v["Members"] = json!(movies.iter().map(|m| format!("movie{m:02}")).collect::<Vec<_>>());
        v["UserData"] = user_data(i + 40, false);
        out.push(v);
    }
    for (i, name) in w().shows.iter().enumerate() {
        let sid = format!("show{i:02}");
        let mut v = common(&sid, name, "Series", i + 50);
        v["Parent"] = json!("libshows");
        v["IsFolder"] = json!(true);
        v["UserData"] = user_data(i + 50, false);
        v["UserData"]["UnplayedItemCount"] = json!(4 + i % 5);
        out.push(v);
        for s in 1..=(2 + i % 2) {
            let season = format!("{sid}s{s}");
            let mut sv = common(&season, &format!("{} {s}", w().season), "Season", i + 50);
            sv["Parent"] = json!(sid);
            sv["SeriesId"] = json!(sid);
            sv["SeriesName"] = json!(name);
            sv["IndexNumber"] = json!(s);
            sv["IsFolder"] = json!(true);
            sv["ImageTags"] = json!({ "Primary": "demo" });
            sv["UserData"] = user_data(i + s, false);
            out.push(sv);
            for e in 1..=(6 + (i + s) % 3) {
                let eid = format!("{season}e{e:02}");
                let titles = w().episodes;
                let mut ev = common(&eid, titles[(e - 1 + i) % titles.len()], "Episode", i * 31 + s * 7 + e);
                ev["Parent"] = json!(season);
                ev["SeriesId"] = json!(sid);
                ev["SeriesName"] = json!(name);
                ev["SeasonId"] = json!(season);
                ev["SeasonName"] = json!(format!("{} {s}", w().season));
                ev["IndexNumber"] = json!(e);
                ev["ParentIndexNumber"] = json!(s);
                ev["SeriesPrimaryImageTag"] = json!("demo");
                ev["ParentLogoItemId"] = json!(sid);
                ev["ParentLogoImageTag"] = json!("demo");
                ev["ParentBackdropItemId"] = json!(sid);
                ev["ParentBackdropImageTags"] = json!(["demo"]);
                ev["ImageTags"] = json!({ "Primary": "demo" });
                ev["BackdropImageTags"] = json!([]);
                playable(&mut ev, &eid, i * 31 + s * 7 + e);
                // Les premiers épisodes de la première saison sont vus : « À suivre » a de quoi proposer.
                ev["UserData"]["Played"] = json!(s == 1 && e <= 1 + i % 3);
                out.push(ev);
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------------------------
// Requêtes
// ---------------------------------------------------------------------------------------------

fn unescape(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < b.len() => {
                match u8::from_str_radix(std::str::from_utf8(&b[i + 1..i + 3]).unwrap_or(""), 16) {
                    Ok(v) => {
                        out.push(v);
                        i += 2;
                    }
                    Err(_) => out.push(b'%'),
                }
            }
            c => out.push(c),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Paramètres de l'adresse, noms en minuscules.
fn params(query: &str) -> HashMap<String, String> {
    query
        .split('&')
        .filter_map(|p| p.split_once('='))
        .map(|(k, v)| (k.to_ascii_lowercase(), unescape(v)))
        .collect()
}

fn lower(s: &str) -> String {
    s.to_lowercase()
}

fn is(v: &Value, key: &str, want: &str) -> bool {
    v[key].as_str() == Some(want)
}

/// Élément tel que le serveur le rend (sans les champs internes du catalogue).
fn public(v: &Value) -> Value {
    let mut v = v.clone();
    if let Some(o) = v.as_object_mut() {
        o.remove("Parent");
        o.remove("Members");
        o.remove("DateCreated");
    }
    v
}

fn list(q: &HashMap<String, String>) -> Value {
    let cat = catalog().lock().unwrap();
    let get = |k: &str| q.get(k).map(String::as_str).filter(|s| !s.is_empty());
    let names = |k: &str| -> Vec<String> { get(k).map(|s| s.split(['|', ',']).map(lower).collect()).unwrap_or_default() };
    let kinds: Vec<String> = get("includeitemtypes").map(|s| s.split(',').map(str::to_string).collect()).unwrap_or_default();
    let recursive = get("recursive").is_some_and(|r| r.eq_ignore_ascii_case("true"));
    let mut items: Vec<&Value> = if let Some(ids) = get("ids") {
        ids.split(',').filter_map(|id| cat.iter().find(|v| is(v, "Id", id))).collect()
    } else if let Some(parent) = get("parentid") {
        match cat.iter().find(|v| is(v, "Id", parent)) {
            Some(p) if p["Members"].is_array() => {
                let members: Vec<&str> = p["Members"].as_array().unwrap().iter().filter_map(|m| m.as_str()).collect();
                cat.iter().filter(|v| members.contains(&v["Id"].as_str().unwrap_or(""))).collect()
            }
            _ => cat.iter().filter(|v| is(v, "Parent", parent)).collect(),
        }
    } else if recursive || !kinds.is_empty() {
        cat.iter().filter(|v| !is(v, "Type", "CollectionFolder")).collect()
    } else {
        Vec::new()
    };
    if !kinds.is_empty() {
        items.retain(|v| kinds.iter().any(|k| is(v, "Type", k)));
    } else if get("ids").is_none() && get("parentid").is_none() {
        items.retain(|v| is(v, "Type", "Movie") || is(v, "Type", "Series"));
    }
    if let Some(term) = get("searchterm").map(lower) {
        items.retain(|v| lower(v["Name"].as_str().unwrap_or("")).contains(&term) || lower(v["SeriesName"].as_str().unwrap_or("")).contains(&term));
    }
    if let Some(p) = get("namestartswith").map(lower) {
        items.retain(|v| lower(v["Name"].as_str().unwrap_or("")).starts_with(&p));
    }
    if let Some(p) = get("namelessthan").map(lower) {
        items.retain(|v| lower(v["Name"].as_str().unwrap_or("")) < p);
    }
    if get("filters").is_some_and(|f| f.contains("IsFavorite")) {
        items.retain(|v| v["UserData"]["IsFavorite"] == true);
    }
    match get("isplayed") {
        Some("false") => items.retain(|v| v["UserData"]["Played"] != true),
        Some("true") => items.retain(|v| v["UserData"]["Played"] == true),
        _ => {}
    }
    if let Some(years) = get("years") {
        let ys: Vec<u64> = years.split(',').filter_map(|y| y.trim().parse().ok()).collect();
        items.retain(|v| v["ProductionYear"].as_u64().is_some_and(|y| ys.contains(&y)));
    }
    if let Some(min) = get("mincommunityrating").and_then(|m| m.parse::<f64>().ok()) {
        items.retain(|v| v["CommunityRating"].as_f64().is_some_and(|r| r >= min));
    }
    if get("imagetypes").is_some_and(|t| t.contains("Backdrop")) {
        items.retain(|v| v["BackdropImageTags"].as_array().is_some_and(|a| !a.is_empty()));
    }
    for (param, field) in [("genres", "Genres"), ("tags", "Tags")] {
        let want = names(param);
        if !want.is_empty() {
            items.retain(|v| v[field].as_array().is_some_and(|a| a.iter().any(|x| want.contains(&lower(x.as_str().unwrap_or(""))))));
        }
    }
    let studios = names("studios");
    if !studios.is_empty() {
        items.retain(|v| studios.contains(&lower(v["Studios"][0]["Name"].as_str().unwrap_or(""))));
    }
    if let Some(id) = get("studioids") {
        items.retain(|v| v["Studios"][0]["Id"].as_str() == Some(id));
    }
    if let Some(id) = get("personids") {
        items.retain(|v| v["People"].as_array().is_some_and(|a| a.iter().any(|p| p["Id"].as_str() == Some(id))));
    }
    let sort = get("sortby").unwrap_or("").to_ascii_lowercase();
    let name_of = |v: &Value| lower(v["Name"].as_str().unwrap_or(""));
    if sort.contains("communityrating") {
        items.sort_by(|a, b| a["CommunityRating"].as_f64().partial_cmp(&b["CommunityRating"].as_f64()).unwrap_or(std::cmp::Ordering::Equal));
    } else if sort.contains("datecreated") {
        // Faux médias mêlés au hasard, les vrais films (DateCreated 1000 et plus) au plus récent.
        items.sort_by_key(|v| {
            let d = v["DateCreated"].as_u64().unwrap_or(0);
            if d >= 1000 { d } else { (d * 7919) % 97 }
        });
    } else if sort.contains("random") {
        items.sort_by_key(|v| (v["DateCreated"].as_u64().unwrap_or(0) * 7919) % 97);
    } else if sort.contains("sortname") || get("namelessthan").is_some() {
        items.sort_by_key(|v| name_of(v));
    } else if get("parentid").is_some() {
        items.sort_by_key(|v| (v["IndexNumber"].as_u64().unwrap_or(0), name_of(v)));
    }
    if get("sortorder").is_some_and(|o| o.eq_ignore_ascii_case("descending")) {
        items.reverse();
    }
    let total = items.len();
    let start: usize = get("startindex").and_then(|s| s.parse().ok()).unwrap_or(0);
    let limit: usize = get("limit").and_then(|s| s.parse().ok()).unwrap_or(usize::MAX);
    let page: Vec<Value> = items.into_iter().skip(start).take(limit).map(public).collect();
    json!({ "Items": page, "TotalRecordCount": total, "StartIndex": start })
}

fn find(id: &str) -> Option<Value> {
    catalog().lock().unwrap().iter().find(|v| is(v, "Id", id)).cloned()
}

fn wrap(items: Vec<Value>) -> Value {
    json!({ "TotalRecordCount": items.len(), "Items": items })
}

/// Prochain épisode non vu de chaque série commencée (ou d'une seule).
fn next_up(series: Option<&str>) -> Value {
    let cat = catalog().lock().unwrap();
    let mut out = Vec::new();
    for show in cat.iter().filter(|v| is(v, "Type", "Series") && series.is_none_or(|s| is(v, "Id", s))) {
        let id = show["Id"].as_str().unwrap_or("");
        let eps: Vec<&Value> = cat.iter().filter(|v| is(v, "Type", "Episode") && is(v, "SeriesId", id)).collect();
        let Some(last) = eps.iter().rposition(|e| e["UserData"]["Played"] == true) else { continue };
        if let Some(next) = eps.get(last + 1) {
            out.push(public(next));
        }
    }
    wrap(out)
}

fn named(list: &[&str], prefix: &str, q: &HashMap<String, String>) -> Value {
    let term = q.get("searchterm").map(|s| lower(s)).unwrap_or_default();
    let limit: usize = q.get("limit").and_then(|s| s.parse().ok()).unwrap_or(usize::MAX);
    wrap(
        list.iter()
            .enumerate()
            .filter(|(_, n)| term.is_empty() || lower(n).contains(&term))
            .take(limit)
            .map(|(i, n)| json!({ "Id": format!("{prefix}{i}"), "Name": n, "Type": if prefix == "person" { "Person" } else { "Studio" } }))
            .collect(),
    )
}

/// Favori / vu : l'état change dans le catalogue, comme sur un vrai serveur.
fn flag(id: &str, field: &str, on: bool) -> Value {
    let mut cat = catalog().lock().unwrap();
    let kids: Vec<String> = cat.iter().filter(|v| is(v, "SeriesId", id) || is(v, "Parent", id)).filter_map(|v| v["Id"].as_str().map(str::to_string)).collect();
    let mut out = json!({});
    for v in cat.iter_mut() {
        let vid = v["Id"].as_str().unwrap_or("").to_string();
        if vid == id || (field == "Played" && kids.contains(&vid)) {
            v["UserData"][field] = json!(on);
            if field == "Played" {
                v["UserData"]["PlayedPercentage"] = Value::Null;
                v["UserData"]["PlaybackPositionTicks"] = json!(0);
            }
            if vid == id {
                out = v["UserData"].clone();
            }
        }
    }
    out
}

fn route(method: &str, path: &str, q: &HashMap<String, String>) -> Reply {
    let parts: Vec<&str> = path.trim_matches('/').split('/').collect();
    let low: Vec<String> = parts.iter().map(|p| p.to_ascii_lowercase()).collect();
    let l: Vec<&str> = low.iter().map(String::as_str).collect();
    let user = json!({
        "Id": USER_ID, "Name": USER, "ServerId": SERVER_ID, "HasPassword": true,
        "Policy": { "IsAdministrator": false, "EnableContentDownloading": false, "EnableRemoteControlOfOtherUsers": false },
        "Configuration": { "EnableNextEpisodeAutoPlay": true, "SubtitleMode": "Default", "PlayDefaultAudioTrack": true },
    });
    match (method, l.as_slice()) {
        ("GET", ["system", "info", "public"]) | ("GET", ["system", "info"]) => {
            Reply::Json(json!({ "Id": SERVER_ID, "ServerName": "Turtlefin Démo", "Version": "10.11.0", "ProductName": "Jellyfin Server", "StartupWizardCompleted": true }))
        }
        ("POST", ["users", "authenticatebyname"]) => Reply::Json(json!({ "AccessToken": "demo", "ServerId": SERVER_ID, "User": user })),
        ("GET", ["users", "me"]) | ("GET", ["users", _]) => Reply::Json(user),
        ("GET", ["userviews"]) | ("GET", ["users", _, "views"]) => {
            Reply::Json(wrap(catalog().lock().unwrap().iter().filter(|v| is(v, "Type", "CollectionFolder")).map(public).collect()))
        }
        ("GET", ["useritems", "resume"]) => Reply::Json(wrap(
            catalog().lock().unwrap().iter().filter(|v| v["UserData"]["PlayedPercentage"].as_f64().is_some_and(|p| p > 0.0)).take(12).map(public).collect(),
        )),
        ("GET", ["shows", "nextup"]) => Reply::Json(next_up(q.get("seriesid").map(String::as_str))),
        ("GET", ["shows", _, "episodes"]) => {
            let cat = catalog().lock().unwrap();
            let season = q.get("seasonid");
            Reply::Json(wrap(
                cat.iter()
                    .filter(|v| is(v, "Type", "Episode") && is(v, "SeriesId", parts[1]) && season.is_none_or(|s| is(v, "SeasonId", s)))
                    .map(public)
                    .collect(),
            ))
        }
        ("GET", ["shows", _, "seasons"]) => {
            Reply::Json(wrap(catalog().lock().unwrap().iter().filter(|v| is(v, "Type", "Season") && is(v, "SeriesId", parts[1])).map(public).collect()))
        }
        ("GET", ["items", "latest"]) => {
            let parent = q.get("parentid").map(String::as_str).unwrap_or("");
            let cat = catalog().lock().unwrap();
            let mut v: Vec<&Value> = cat.iter().filter(|v| is(v, "Parent", parent)).collect();
            v.reverse();
            Reply::Json(Value::Array(v.into_iter().take(16).map(public).collect()))
        }
        ("GET", ["items", "filters2"]) | ("GET", ["items", "filters"]) => Reply::Json(json!({
            "Genres": w().genres.iter().enumerate().map(|(i, g)| json!({ "Name": g, "Id": format!("genre{i}") })).collect::<Vec<_>>(),
            "Tags": w().tags,
        })),
        ("GET", ["studios"]) => Reply::Json(named(&w().studios, "studio", q)),
        ("GET", ["persons"]) => Reply::Json(named(&PEOPLE, "person", q)),
        ("GET", ["items"]) | ("GET", ["users", _, "items"]) => Reply::Json(list(q)),
        ("GET", ["items", _, "similar"]) => {
            let kind = find(parts[1]).map(|v| v["Type"].as_str().unwrap_or("Movie").to_string()).unwrap_or_default();
            let kind = if kind == "Movie" || kind == "Series" { kind } else { "Series".to_string() };
            let cat = catalog().lock().unwrap();
            let seed = parts[1].bytes().map(|b| b as usize).sum::<usize>();
            let pool: Vec<&Value> = cat.iter().filter(|v| is(v, "Type", &kind) && !is(v, "Id", parts[1])).collect();
            Reply::Json(wrap((0..pool.len().min(12)).map(|i| public(pool[(seed + i) % pool.len()])).collect()))
        }
        ("GET", ["items", _, "ancestors"]) => {
            let mut out = Vec::new();
            let mut cur = find(parts[1]);
            while let Some(p) = cur.and_then(|v| v["Parent"].as_str().and_then(find)) {
                out.push(public(&p));
                cur = Some(p);
            }
            Reply::Json(Value::Array(out))
        }
        ("GET", ["items", _, "localtrailers"]) | ("GET", ["sessions"]) | ("GET", ["playlists", ..]) => Reply::Json(json!([])),
        ("GET", ["items", _, "images", ..]) => match find(parts[1]) {
            Some(v) => image(&v, l[3], q),
            None => Reply::NotFound,
        },
        ("GET", ["items", _]) | ("GET", ["users", _, "items", _]) => match find(parts[parts.len() - 1]) {
            Some(v) => Reply::Json(public(&v)),
            None => Reply::NotFound,
        },
        ("POST", ["items", _, "playbackinfo"]) => Reply::Json(json!({ "MediaSources": [media_source(parts[1])], "PlaySessionId": "demo" })),
        ("GET" | "HEAD", ["videos", _, ..]) => Reply::Video,
        ("POST" | "DELETE", ["userfavoriteitems", _]) => Reply::Json(flag(parts[1], "IsFavorite", method == "POST")),
        ("POST" | "DELETE", ["userplayeditems", _]) => Reply::Json(flag(parts[1], "Played", method == "POST")),
        ("POST" | "DELETE", _) => Reply::Empty,
        _ => Reply::NotFound,
    }
}

enum Reply {
    Json(Value),
    Png(Vec<u8>),
    Jpeg(Vec<u8>),
    Video,
    Empty,
    NotFound,
}

fn serve(stream: TcpStream) -> std::io::Result<()> {
    stream.set_read_timeout(Some(std::time::Duration::from_secs(10)))?;
    // Requête bornée (ligne, en-têtes : 64 Kio ; corps : 1 Mio) : n'importe quel programme de la
    // machine, ou une page web, peut écrire sur 127.0.0.1.
    let mut reader = BufReader::new(stream.try_clone()?.take(64 << 10));
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let mut it = line.split_whitespace();
    let (method, target) = (it.next().unwrap_or("").to_string(), it.next().unwrap_or("/").to_string());
    let (mut length, mut range) = (0usize, None::<(u64, Option<u64>)>);
    loop {
        let mut h = String::new();
        if reader.read_line(&mut h)? == 0 || h.trim().is_empty() {
            break;
        }
        let Some((k, v)) = h.split_once(':') else { continue };
        match k.to_ascii_lowercase().as_str() {
            "content-length" => length = v.trim().parse().unwrap_or(0),
            "range" => {
                if let Some((a, b)) = v.trim().trim_start_matches("bytes=").split_once('-') {
                    range = a.trim().parse().ok().map(|a| (a, b.trim().parse().ok()));
                }
            }
            _ => {}
        }
    }
    reader.get_mut().set_limit(1 << 20);
    let mut body = vec![0u8; length.min(1 << 20)];
    reader.read_exact(&mut body)?;
    let (path, query) = target.split_once('?').unwrap_or((&target, ""));
    let mut out = stream;
    let head = |code: &str, kind: &str, len: u64, extra: &str| format!("HTTP/1.1 {code}\r\nContent-Type: {kind}\r\nContent-Length: {len}\r\nConnection: close\r\n{extra}\r\n");
    match route(&method, &unescape(path), &params(query)) {
        Reply::Json(v) => {
            let b = serde_json::to_vec(&v).unwrap_or_default();
            out.write_all(head("200 OK", "application/json; charset=utf-8", b.len() as u64, "").as_bytes())?;
            out.write_all(&b)
        }
        Reply::Png(b) => {
            out.write_all(head("200 OK", "image/png", b.len() as u64, "").as_bytes())?;
            out.write_all(&b)
        }
        Reply::Jpeg(b) => {
            out.write_all(head("200 OK", "image/jpeg", b.len() as u64, "").as_bytes())?;
            out.write_all(&b)
        }
        Reply::Empty => out.write_all(head("204 No Content", "text/plain", 0, "").as_bytes()),
        Reply::NotFound => out.write_all(head("404 Not Found", "text/plain", 0, "").as_bytes()),
        Reply::Video => {
            let total = video_len();
            let (from, to) = match range {
                // Plage à l'envers (« bytes=100-50 ») : vide, sans débordement.
                Some((a, b)) => {
                    let from = a.min(total);
                    (from, b.map(|b| b.saturating_add(1)).unwrap_or(total).min(total).max(from))
                }
                None => (0, total),
            };
            let extra = format!("Accept-Ranges: bytes\r\n{}", if range.is_some() { format!("Content-Range: bytes {from}-{}/{total}\r\n", to.saturating_sub(1)) } else { String::new() });
            let code = if range.is_some() { "206 Partial Content" } else { "200 OK" };
            out.write_all(head(code, "video/x-yuv4mpeg", to - from, &extra).as_bytes())?;
            if method == "HEAD" {
                return Ok(());
            }
            video_write(&mut out, from, to)
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Vidéo : une mire animée au format YUV4MPEG2 (images brutes, lues par mpv sans décodeur), fabriquée
// à la demande : chaque octet se calcule depuis sa position, d'où les sauts dans la vidéo.
// ---------------------------------------------------------------------------------------------

fn video_head() -> Vec<u8> {
    format!("YUV4MPEG2 W{VIDEO_W} H{VIDEO_H} F{VIDEO_FPS}:1 Ip A1:1 C420jpeg\n").into_bytes()
}

const FRAME: usize = 6 + VIDEO_W * VIDEO_H * 3 / 2;

fn video_len() -> u64 {
    video_head().len() as u64 + VIDEO_SECS * VIDEO_FPS * FRAME as u64
}

fn video_frame(n: u64) -> Vec<u8> {
    let mut f = Vec::with_capacity(FRAME);
    f.extend_from_slice(b"FRAME\n");
    let t = n as f32 / VIDEO_FPS as f32;
    // Luminance : dégradé qui ondule, disque clair qui traverse l'image, barre d'avancement en bas.
    let (cx, cy) = (VIDEO_W as f32 * (0.5 + 0.35 * (t * 0.7).sin()), VIDEO_H as f32 * (0.5 + 0.25 * (t * 1.1).cos()));
    let done = (n as f32 / (VIDEO_SECS * VIDEO_FPS) as f32 * VIDEO_W as f32) as usize;
    for y in 0..VIDEO_H {
        for x in 0..VIDEO_W {
            let wave = ((x as f32 * 0.03 + t).sin() + (y as f32 * 0.05 - t * 0.6).cos()) * 22.0;
            let d = ((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)).sqrt();
            let disc = (1.0 - (d / 34.0)).clamp(0.0, 1.0) * 120.0;
            let mut v = 70.0 + wave + disc + y as f32 * 0.2;
            if y >= VIDEO_H - 6 {
                v = if x < done { 225.0 } else { 40.0 };
            }
            f.push(v.clamp(16.0, 235.0) as u8);
        }
    }
    // Couleur : teinte qui tourne lentement, différente à gauche et à droite.
    for plane in 0..2 {
        for y in 0..VIDEO_H / 2 {
            for x in 0..VIDEO_W / 2 {
                let a = t * 0.25 + x as f32 / VIDEO_W as f32 * 2.0 + y as f32 / VIDEO_H as f32;
                let c = if plane == 0 { a.cos() } else { a.sin() };
                f.push((128.0 + c * 46.0) as u8);
            }
        }
    }
    f
}

fn video_write(out: &mut impl Write, from: u64, to: u64) -> std::io::Result<()> {
    let head = video_head();
    let mut pos = from;
    if pos < head.len() as u64 {
        let end = (head.len() as u64).min(to);
        out.write_all(&head[pos as usize..end as usize])?;
        pos = end;
    }
    while pos < to {
        let rel = pos - head.len() as u64;
        let (n, off) = (rel / FRAME as u64, (rel % FRAME as u64) as usize);
        let frame = video_frame(n);
        let end = (FRAME as u64 - off as u64).min(to - pos) as usize + off;
        out.write_all(&frame[off..end])?;
        pos += (end - off) as u64;
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Images : affiches, fonds et logos dessinés (dégradé propre à chaque titre, titre en lettres à points).
// ---------------------------------------------------------------------------------------------

/// Lettres 5 x 7 (A à Z), une ligne par octet.
const FONT: [[u8; 7]; 26] = [
    [0x0E, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11], [0x1E, 0x11, 0x11, 0x1E, 0x11, 0x11, 0x1E], [0x0E, 0x11, 0x10, 0x10, 0x10, 0x11, 0x0E],
    [0x1E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x1E], [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x1F], [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x10],
    [0x0E, 0x11, 0x10, 0x17, 0x11, 0x11, 0x0F], [0x11, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11], [0x0E, 0x04, 0x04, 0x04, 0x04, 0x04, 0x0E],
    [0x07, 0x02, 0x02, 0x02, 0x02, 0x12, 0x0C], [0x11, 0x12, 0x14, 0x18, 0x14, 0x12, 0x11], [0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x1F],
    [0x11, 0x1B, 0x15, 0x15, 0x11, 0x11, 0x11], [0x11, 0x19, 0x15, 0x13, 0x11, 0x11, 0x11], [0x0E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
    [0x1E, 0x11, 0x11, 0x1E, 0x10, 0x10, 0x10], [0x0E, 0x11, 0x11, 0x11, 0x15, 0x12, 0x0D], [0x1E, 0x11, 0x11, 0x1E, 0x14, 0x12, 0x11],
    [0x0F, 0x10, 0x10, 0x0E, 0x01, 0x01, 0x1E], [0x1F, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04], [0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
    [0x11, 0x11, 0x11, 0x11, 0x11, 0x0A, 0x04], [0x11, 0x11, 0x11, 0x15, 0x15, 0x1B, 0x11], [0x11, 0x11, 0x0A, 0x04, 0x0A, 0x11, 0x11],
    [0x11, 0x11, 0x0A, 0x04, 0x04, 0x04, 0x04], [0x1F, 0x01, 0x02, 0x04, 0x08, 0x10, 0x1F],
];

/// Chiffres 5 x 7 (0 à 9), même dessin que les lettres.
const DIGITS: [[u8; 7]; 10] = [
    [0x0E, 0x11, 0x13, 0x15, 0x19, 0x11, 0x0E], [0x04, 0x0C, 0x04, 0x04, 0x04, 0x04, 0x0E], [0x0E, 0x11, 0x01, 0x02, 0x04, 0x08, 0x1F],
    [0x1F, 0x02, 0x04, 0x02, 0x01, 0x11, 0x0E], [0x02, 0x06, 0x0A, 0x12, 0x1F, 0x02, 0x02], [0x1F, 0x10, 0x1E, 0x01, 0x01, 0x11, 0x0E],
    [0x06, 0x08, 0x10, 0x1E, 0x11, 0x11, 0x0E], [0x1F, 0x01, 0x02, 0x04, 0x08, 0x08, 0x08], [0x0E, 0x11, 0x11, 0x0E, 0x11, 0x11, 0x0E],
    [0x0E, 0x11, 0x11, 0x0F, 0x01, 0x02, 0x0C],
];

/// Dessin d'un caractère (lettre ou chiffre).
fn glyph(c: char) -> Option<[u8; 7]> {
    c.to_digit(10).map(|d| DIGITS[d as usize]).or_else(|| letter(c).map(|i| FONT[i]))
}

fn letter(c: char) -> Option<usize> {
    let c = match c {
        'à' | 'â' | 'ä' | 'À' | 'Â' => 'a',
        'é' | 'è' | 'ê' | 'ë' | 'É' | 'È' | 'Ê' => 'e',
        'î' | 'ï' | 'Î' => 'i',
        'ô' | 'ö' | 'Ô' => 'o',
        'ù' | 'û' | 'ü' => 'u',
        'ç' => 'c',
        c => c,
    };
    c.is_ascii_alphabetic().then(|| (c.to_ascii_uppercase() as u8 - b'A') as usize)
}

/// Lignes d'un titre qui tiennent en `cols` lettres.
fn wrap_title(title: &str, cols: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for word in title.split([' ', '\'', '-']).filter(|w| !w.is_empty()) {
        match lines.last_mut() {
            Some(l) if l.chars().count() + 1 + word.chars().count() <= cols => {
                l.push(' ');
                l.push_str(word);
            }
            _ => lines.push(word.to_string()),
        }
    }
    lines
}

/// Écrit `title` centré dans la zone (x, y, w, h), le plus gros possible.
fn draw_title(img: &mut image::RgbaImage, title: &str, zone: (u32, u32, u32, u32), color: [u8; 4]) {
    let (zx, zy, zw, zh) = zone;
    let lines = wrap_title(title, if zw > zh * 3 { 22 } else { 10 });
    let longest = lines.iter().map(|l| l.chars().count()).max().unwrap_or(1).max(1) as u32;
    let s = (zw / (longest * 6)).min(zh / (lines.len() as u32 * 9).max(1)).max(1);
    let total_h = lines.len() as u32 * 9 * s;
    for (li, line) in lines.iter().enumerate() {
        let w = line.chars().count() as u32 * 6 * s;
        let x0 = zx + zw.saturating_sub(w) / 2;
        let y0 = zy + zh.saturating_sub(total_h) / 2 + li as u32 * 9 * s;
        for (ci, ch) in line.chars().enumerate() {
            let Some(g) = glyph(ch) else { continue };
            for (row, bits) in g.iter().enumerate() {
                for col in 0..5 {
                    if bits & (0x10 >> col) == 0 {
                        continue;
                    }
                    for dy in 0..s {
                        for dx in 0..s {
                            let (x, y) = (x0 + (ci as u32 * 6 + col) * s + dx, y0 + row as u32 * s + dy);
                            if x < img.width() && y < img.height() {
                                img.put_pixel(x, y, image::Rgba(color));
                            }
                        }
                    }
                }
            }
        }
    }
}

fn hsv(h: f32, s: f32, v: f32) -> [f32; 3] {
    let h = h.rem_euclid(360.0) / 60.0;
    let (c, x) = (v * s, v * s * (1.0 - (h % 2.0 - 1.0).abs()));
    let (r, g, b) = match h as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    [(r + m) * 255.0, (g + m) * 255.0, (b + m) * 255.0]
}

fn image(item: &Value, kind: &str, q: &HashMap<String, String>) -> Reply {
    let has = |k: &str| item["ImageTags"].get(k).is_some();
    let ok = match kind {
        "primary" => has("Primary"),
        "logo" => has("Logo"),
        "thumb" => has("Thumb"),
        "backdrop" => item["BackdropImageTags"].as_array().is_some_and(|a| !a.is_empty()),
        _ => false,
    };
    if !ok {
        return Reply::NotFound;
    }
    let num = |k: &str| q.get(k).and_then(|v| v.parse::<u32>().ok());
    // Vrai film : son affiche ou son image, à la taille demandée.
    if let Some(f) = film_of(item) {
        let src = if kind == "primary" { f.poster } else { f.backdrop };
        let Ok(img) = image::load_from_memory(src) else { return Reply::NotFound };
        let w = num("fillwidth").or(num("maxwidth")).unwrap_or(img.width()).clamp(32, img.width());
        let h = num("fillheight").unwrap_or(w * img.height() / img.width()).clamp(32, img.height());
        let img = img.resize_to_fill(w, h, image::imageops::FilterType::Triangle).to_rgb8();
        let mut out = Vec::new();
        let enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 88);
        return match image::ImageEncoder::write_image(enc, img.as_raw(), img.width(), img.height(), image::ExtendedColorType::Rgb8) {
            Ok(()) => Reply::Jpeg(out),
            Err(_) => Reply::NotFound,
        };
    }
    let name = item["Name"].as_str().unwrap_or("");
    let title = item["SeriesName"].as_str().filter(|_| item["Type"] == "Season").unwrap_or(name);
    let wide = kind != "primary" || item["Type"] == "Episode" || item["Type"] == "CollectionFolder";
    let seed = item["Id"].as_str().unwrap_or("").bytes().fold(7u32, |a, b| a.wrapping_mul(31).wrapping_add(b as u32));
    let hue = (seed % 360) as f32;
    let mut out = Vec::new();
    let img = if kind == "logo" {
        let w = num("maxwidth").unwrap_or(600).clamp(120, 900);
        let mut img = image::RgbaImage::new(w, w / 4);
        draw_title(&mut img, title, (0, 0, w, w / 4), [255, 255, 255, 255]);
        img
    } else {
        let (dw, dh) = if wide { (640, 360) } else { (400, 600) };
        let w = num("fillwidth").or(num("maxwidth")).unwrap_or(dw).clamp(32, 1280);
        let h = num("fillheight").unwrap_or(w * dh / dw).clamp(32, 1280);
        let (a, b) = (hsv(hue, 0.62, 0.55), hsv(hue + 70.0, 0.7, 0.16));
        let (cx, cy, r) = (w as f32 * (0.25 + (seed % 50) as f32 / 100.0), h as f32 * 0.34, w.min(h) as f32 * 0.3);
        let glow = hsv(hue - 40.0, 0.35, 1.0);
        let mut img = image::RgbaImage::from_fn(w, h, |x, y| {
            let t = (y as f32 / h as f32 * 0.8 + x as f32 / w as f32 * 0.2).clamp(0.0, 1.0);
            let d = ((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)).sqrt();
            let g = (1.0 - d / r).clamp(0.0, 1.0).powi(2) * 0.75;
            let band = if ((x + y) / (w / 9).max(1)) % 2 == 0 { 1.0 } else { 0.93 };
            let c = |i: usize| (((a[i] + (b[i] - a[i]) * t) * band) * (1.0 - g) + glow[i] * g) as u8;
            image::Rgba([c(0), c(1), c(2), 255])
        });
        if kind == "primary" {
            let label = if item["Type"] == "Season" { format!("{title} {name}") } else { title.to_string() };
            draw_title(&mut img, &label, (w / 10, h * 55 / 100, w * 8 / 10, h * 35 / 100), [255, 255, 255, 235]);
        }
        img
    };
    let enc = image::codecs::png::PngEncoder::new_with_quality(&mut out, image::codecs::png::CompressionType::Fast, image::codecs::png::FilterType::NoFilter);
    match image::ImageEncoder::write_image(enc, img.as_raw(), img.width(), img.height(), image::ExtendedColorType::Rgba8) {
        Ok(()) => Reply::Png(out),
        Err(_) => Reply::NotFound,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(s: &str) -> HashMap<String, String> {
        params(s)
    }

    #[test]
    fn library_pages_and_index() {
        let page = list(&q("parentId=libmovies&IncludeItemTypes=Movie&Recursive=true&SortBy=SortName&StartIndex=0&Limit=10"));
        assert_eq!(page["TotalRecordCount"], 30 + FILMS.len());
        assert_eq!(page["Items"].as_array().unwrap().len(), 10);
        let before = list(&q("parentId=libmovies&Limit=0&NameLessThan=M"));
        assert!(before["TotalRecordCount"].as_u64().unwrap() > 5);
        assert!(before["Items"].as_array().unwrap().is_empty());
    }

    #[test]
    fn seasons_episodes_and_next_up() {
        let seasons = list(&q("parentId=show00"));
        assert_eq!(seasons["Items"].as_array().unwrap().len(), 2);
        let eps = list(&q("parentId=show00s1"));
        assert!(eps["Items"].as_array().unwrap().len() >= 6);
        let next = next_up(Some("show00"));
        assert_eq!(next["Items"][0]["IndexNumber"], 2);
        // Les éléments rendus se lisent comme ceux d'un vrai serveur.
        let item: crate::api::Item = serde_json::from_value(public(&find("show00s1e01").unwrap())).unwrap();
        assert_eq!(item.kind, "Episode");
        assert_eq!(item.series_id.as_deref(), Some("show00"));
    }

    #[test]
    fn search_filters_and_flags() {
        assert!(!list(&q("Recursive=true&searchTerm=phare&IncludeItemTypes=Movie,Series"))["Items"].as_array().unwrap().is_empty());
        let horror = list(&q("Recursive=true&IncludeItemTypes=Movie,Series&Tags=Survival+Horror"));
        assert!(!horror["Items"].as_array().unwrap().is_empty());
        assert_eq!(flag("movie03", "IsFavorite", true)["IsFavorite"], true);
        assert!(list(&q("Recursive=true&Filters=IsFavorite&IncludeItemTypes=Movie"))["Items"].as_array().unwrap().iter().any(|v| v["Id"] == "movie03"));
    }

    #[test]
    fn video_bytes_are_addressable() {
        let mut all = Vec::new();
        video_write(&mut all, 0, 200_000).unwrap();
        let mut part = Vec::new();
        video_write(&mut part, 90_000, 150_000).unwrap();
        assert_eq!(&all[90_000..150_000], &part[..]);
        assert!(all.starts_with(b"YUV4MPEG2 "));
    }

    #[test]
    fn images_are_png() {
        let Reply::Png(b) = image(&find("movie00").unwrap(), "primary", &q("fillWidth=270&fillHeight=405")) else { panic!() };
        let img = image::load_from_memory(&b).unwrap();
        assert_eq!((img.width(), img.height()), (270, 405));
        assert!(matches!(image(&find("movie00").unwrap(), "logo", &q("maxWidth=400")), Reply::Png(_)));
    }

    #[test]
    fn every_item_reads_like_a_real_one() {
        // Chaque fiche du catalogue se lit comme celle d'un vrai serveur (une seule qui échoue : la
        // fiche ne s'ouvre pas pendant la visite).
        let all: Vec<Value> = catalog().lock().unwrap().clone();
        for v in all {
            let id = v["Id"].as_str().unwrap_or("").to_string();
            let Reply::Json(j) = route("GET", &format!("/Items/{id}"), &q("userId=x")) else { panic!("{id} : introuvable") };
            if let Err(e) = serde_json::from_value::<crate::api::Item>(j) {
                panic!("{id} : {e}");
            }
        }
    }

    #[test]
    fn films_have_real_images() {
        let Reply::Jpeg(b) = image(&find("film-bbb").unwrap(), "primary", &q("fillWidth=270&fillHeight=405")) else { panic!() };
        let img = image::load_from_memory(&b).unwrap();
        assert_eq!((img.width(), img.height()), (270, 405));
        assert!(matches!(image(&find("film-sintel").unwrap(), "backdrop", &q("maxWidth=1280")), Reply::Jpeg(_)));
        assert!(matches!(image(&find("film-sintel").unwrap(), "logo", &q("maxWidth=400")), Reply::NotFound));
        let latest = list(&q("parentId=libmovies&SortBy=DateCreated&SortOrder=Descending&Limit=3"));
        assert_eq!(latest["Items"][0]["Id"], "film-sprite");
    }
}
