use libc::fork;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, process::{self, Command}, time::Duration};
use x11rb::protocol::xproto::ConnectionExt;
use x11_clipboard::{Atom, Clipboard as X11Clipboard};

#[derive(Clone, Serialize, Deserialize)]
pub struct Contents (HashMap<String, Vec<u8>>);

impl Contents {
    fn new(targets: Vec<&str>) -> Result<Self, String> {
        let mut map = HashMap::new();

        for target in targets {
            if target != "TARGETS" {
                let contents = Command::new("xclip")
                    .arg("-o")
                    .arg("-target")
                    .arg(target)
                    .arg("-selection")
                    .arg("clipboard")
                    .output()
                    .map_err(|e| e.to_string())?
                    .stdout;
                map.insert(target.to_string(), contents);
            }
        }

        Ok(Self(map))
    }
}

pub struct Clipboard;

impl Clipboard {
    pub fn get_contents() -> Result<Contents, String> {
        let targets = Command::new("xclip")
            .arg("-o")
            .arg("-target")
            .arg("TARGETS")
            .arg("-selection")
            .arg("clipboard")
            .output()
            .map_err(|e| e.to_string())?
            .stdout;
        let targets = String::from_utf8(targets)
            .map_err(|e| e.to_string())?;
        let targets = targets
            .split_whitespace()
            .collect();

        Contents::new(targets)
    }

    pub fn set_contents(contents: Contents) -> Result<(), String> {
        match unsafe { fork() } {
            -1 => Err(format!("Could not fork process")),
            0 => {
                // Obtain new X11 clipboard context
                let clip = X11Clipboard::new().expect("Failed to obtain X11 clipboard context");

                // Load contents into clipboard
                for kv in contents.0 {
                    clip.store(
                        clip.setter.atoms.clipboard,
                        clip.getter.get_atom(kv.0.as_str()).expect("Failed to obtain atom for a saved target"),
                        kv.1,
                    ).expect("Failed to set clipboard contents through forked process");
                }

                // Wait for clipboard to change, then kill fork
                loop {
                    if clip.setter.connection.get_selection_owner(clip.getter.atoms.clipboard)
                        .expect("Failed to obtain current X11 clipboard owner")
                        .reply()
                        .map(|reply| reply.owner != clip.setter.window)
                        .unwrap_or(true)
                    {
                        process::exit(0);
                    }
                }
            }
            _pid => Ok(()),
        }
    }
}