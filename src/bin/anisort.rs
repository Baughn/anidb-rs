use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::iter::FromIterator;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

extern crate anidb;
use anidb::ed2k::Ed2kHash;
use anidb::{Anidb, AnidbError, File};

extern crate rayon;
use rayon::prelude::*;

extern crate app_dirs;
use app_dirs::*;

extern crate ini;
use ini::Ini;

// Config data:
const APP_INFO: AppInfo = AppInfo { name: "anisort", author: "Baughn" };

struct ConfigData {
    user: String,
    password: String,
    target: PathBuf,
}

impl ConfigData {
    fn initialize_file<T>(file: &PathBuf) -> T {
        let mut ini = Ini::new();
        ini.with_section(Some("User"))
            .set("username", "<USERNAME>")
            .set("password", "<PASSWORD>");
        ini.with_section(Some("Target directories"))
            .set("target", env::home_dir().unwrap().join("Anime").to_string_lossy());
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        ini.write_to_file(file).expect("Failed to write ini file!");
        panic!("Ini file created. Fill in the template in {:?}", file);
    }
    
    pub fn from_file(file: PathBuf) -> Option<ConfigData> {
        let ini = Ini::load_from_file(&file).unwrap_or_else(|_| ConfigData::initialize_file(&file));
        let user_section = ini.section(Some("User"))?;
        let dirs = ini.section(Some("Target directories"))?;
        let user = user_section.get("username")?;
        let password = user_section.get("password")?;
        let target = dirs.get("target")?;
        return Some(ConfigData {
            user: user.to_string(),
            password: password.to_string(),
            target: PathBuf::from(target),
        });
    }
}


fn walk_dir(path: &Path) -> BTreeSet<PathBuf> {
    let mut set = BTreeSet::new();
    let ftype = path.symlink_metadata().expect("walk_dir meta").file_type();
    if ftype.is_symlink() {
        // Skip.
    } else if ftype.is_dir() {
        for entry in fs::read_dir(path).expect("walk_dir read") {
            let entry = entry.expect("walk_dir entry");
            let next = entry.path();
            if next.is_dir() {
                set.append(&mut walk_dir(&next));
            } else {
                set.insert(entry.path());
            }
        }
    } else {
        assert!(ftype.is_file());
        set.insert(path.to_path_buf());
    }
    return set;
}

/// All the data we could ever want about hashed files...
#[derive(Debug)]
struct HashData {
    filename: PathBuf,
    hash: Result<Ed2kHash, AnidbError>,
}

fn hash(filename: PathBuf) -> HashData {
    let hash = Ed2kHash::from_file(&filename);
    return HashData {
        filename: filename,
        hash: hash
    };
}

fn clean(raw: &String) -> String {
    return raw.replace(" ", "_").replace("/", "|");
}

fn build_path(file: &File, hashdata: &HashData, target_dir: &PathBuf) -> PathBuf {
    let series = &file.series_romaji;
    assert!(series != "");
    let mut new_name = format!("{} - ", series);
    // Episode number.
    let ep_num_int: std::result::Result<u32, _> = file.ep_number.parse();
    if ep_num_int.is_ok() {
        for _ in (file.ep_number.len())..(format!("{}", file.total_eps).len()) {
            new_name.push('0');
        }
    }
    new_name.push_str(&file.ep_number);
    // Episode name.
    let ep_name = &file.ep_name;
    assert!(ep_name != "");
    new_name.push_str(&format!(" {}", ep_name));
    // Extension.
    let ext = hashdata.filename.extension().expect("Extension").to_str().expect("to_str");
    new_name.push('.');
    new_name.push_str(ext);
    // Build the final path.
    let full_path = target_dir
        .join(clean(&file.series_romaji))
        .join(clean(&new_name));

    return full_path;
}

fn move_file(mode_noop: bool, from: &PathBuf, to: &PathBuf) {
    if mode_noop {
        println!("Would move \
                  {:?} \
                  to \
                  {:?}", from, to);
    } else if from == to {
        println!("Not moving {:?}", from);
    } else {
        println!("Moving {:?}", from);
        println!("    to {:?}", to);
        fs::create_dir_all(to.parent().unwrap()).expect("create_dir_all");
        if let Err(_) = fs::rename(from, to) {
            fs::copy(from, to).expect("Copy");
            fs::remove_file(from).expect("Delete old");
        }
    }
}

fn search(db: &Arc<Mutex<Anidb>>, mode_noop: bool, hashdata: HashData, target_dir: &PathBuf) -> () {
    match hashdata.hash {
        Ok(ref hash) => {
            let result = db.lock().expect("lock").file_from_hash(&hash);
            match result {
                Ok(file) => {
                    let new_path = build_path(&file, &hashdata, target_dir);
                    move_file(mode_noop, &hashdata.filename, &new_path);
                },
                Err(err) => {
                    println!("Looking up {:?}: {}", hashdata.filename, err);
                },
            };
        },
        Err(err) => {
            println!("Looking up {:?}: {}", hashdata.filename, err);
        }
    };
}

fn main() -> () {
    let config_dir = get_app_root(AppDataType::UserConfig, &APP_INFO).expect("Failed to get app dir");
    let cache_dir = get_app_root(AppDataType::UserCache, &APP_INFO).expect("Failed to get cache dir");
    let config = ConfigData::from_file(config_dir.join("config.ini")).expect("Failed to load config file");
    
    // Parse command line for parameters.
    let mut args : BTreeSet<String> = BTreeSet::from_iter(env::args().skip(1));
    let mode_noop = args.remove("-n");

    // Login to AniDB.
    let db = Arc::new(Mutex::new(Anidb::new(("api.anidb.net", 9000), &cache_dir).unwrap()));
    db.lock().unwrap().login(&config.user, &config.password).expect("Failed AniDB login");

    // List all files, hash and send them...
    args.par_iter()
        .flat_map(|filename| walk_dir(Path::new(&filename).canonicalize().unwrap().as_path()))
        .map(|file| hash(file))
        .for_each(|hashdata| search(&db, mode_noop, hashdata, &config.target));
}
