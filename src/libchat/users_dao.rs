use std::{
    collections::{hash_map::Entry, HashMap},
    fmt::{self, Debug},
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
};

use regex::Regex;

use crate::err::MyResult;

pub struct UsersDao {
    path: PathBuf,
    users: HashMap<String, String>,
    dirty: bool,
}

impl Drop for UsersDao {
    fn drop(&mut self) {
        if !self.dirty {
            return;
        }

        let mut f = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&self.path)
            .unwrap_or_else(|e| {
                panic!("failed to write users database file: {}", e)
            });

        for e in self.users.iter() {
            f.write_all(format!("({}, {})\n", e.0, e.1).as_bytes())
                .expect("failed to write to users database file");
        }
    }
}

impl UsersDao {
    pub fn from(path_ref: impl AsRef<Path>) -> MyResult<Self> {
        let path = path_ref.as_ref().to_path_buf();

        if !path.exists() {
            return Err(format!(
                "no such users database file: {}",
                path.display()
            )
            .into());
        }
        if !path.is_file() {
            return Err(format!(
                "users database file is not a regular file: {}",
                path.display()
            )
            .into());
        }

        let mut users = HashMap::<String, String>::new();

        let reader = BufReader::new(File::open(&path)?);
        let line_re = Regex::new(r"^\s*\(\s*([^,]+)\s*,\s*([^)]+)\s*\)\s*$")?;

        for (line_no, line_res) in reader.lines().enumerate() {
            let line = line_res?;
            if let Some(m) = line_re.captures(&line) {
                let username = m.get(1).unwrap().as_str().to_owned();
                let password = m.get(2).unwrap().as_str();
                // TODO: error if duplicate username found
                users.entry(username).or_default().push_str(password);
            } else {
                return Err(format!(
                    "invalid line in users database: {}:{}:{}",
                    path.display(),
                    line_no,
                    line
                )
                .into());
            }
        }

        Ok(Self {
            path,
            users,
            dirty: false,
        })
    }

    pub fn entry(&mut self, user: impl AsRef<str>) -> Entry<String, String> {
        self.users.entry(user.as_ref().to_string())
    }

    pub fn insert<S: AsRef<str>>(&mut self, user: S, pass: S) -> bool {
        self.dirty = true;
        match self.users.entry(user.as_ref().to_string()) {
            Entry::Occupied(_) => return false,
            Entry::Vacant(ve) => {
                ve.insert(pass.as_ref().to_string());
            }
        }
        true
    }
}

impl Debug for UsersDao {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_fmt(format_args!("{:?}", self.users))
    }
}
