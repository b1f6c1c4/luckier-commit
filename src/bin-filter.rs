use luckier_commit::{HashPrefix, HashSearchWorker, Sha1};

use std::collections::VecDeque;

use std::io::{prelude::*, stdin, BufReader};

use std::process::{Command, Stdio};

fn main() -> std::io::Result<()> {
    let stdin = stdin();
    let mut reader = BufReader::new(stdin);

    let mut hashes = VecDeque::<(String, usize)>::new();

    #[derive(Debug)]
    enum State {
        Idle,
        RefSelected,
        Committing,
        Tagging,
        Frommed,
    }
    let mut state = State::Idle;
    let mut refspec = String::new();
    let mut lucky: usize = 0;
    let mut original = String::new();
    let mut tree = String::new();
    let mut parents = String::new();
    let mut author = String::new();
    let mut committer = String::new();
    let mut mark: usize = 0;
    let mut message: Vec<u8> = Vec::new();
    'a: loop {
        let mut line = String::new();
        reader.read_line(&mut line)?;
        match line.as_str() {
            "feature done\n" => print!("{}", line),
            "done\n" => {
                println!("done");
                break 'a;
            }
            "\n" => {
                let (ty, info) = match state {
                    State::Committing => ("commit", format!("tree {}{}{}{}\n", tree, parents, author, committer)),
                    State::Tagging => ("tag", format!("{}type commit\n{}{}\n", parents, author, committer)),
                    State::Frommed => {
                        state = State::RefSelected;
                        continue 'a;
                    }
                    _ => panic!("Illegal empty line in {:?} state", state),
                };
                let existing_commit = [info.as_bytes(), message.as_slice()].concat();
                let prefix_spec = format!("{:07x}", lucky);
                let desired_prefix = Some(&prefix_spec)
                    .map(String::as_str)
                    .map(str::parse::<HashPrefix<Sha1>>)
                    .transpose()
                    .unwrap()
                    .unwrap();
                println!("progress Luckilizing {} {} of height {} ... ", ty, original, lucky);
                let found_commit =
                    HashSearchWorker::new(existing_commit.as_slice(), desired_prefix)
                        .search()
                        .unwrap();
                let new_hash = found_commit.hex_hash();
                let hash = new_hash.to_string();
                let new_git_oid = spawn_git(
                    &["hash-object", "-t", ty, "-w", "--stdin"],
                    Some(found_commit.object()),
                );
                assert_eq!(
                    new_hash.as_bytes(),
                    &new_git_oid[0..new_hash.len()],
                    "Found a matching commit, but git unexpectedly computed a different hash for it",
                );
                while hashes.len() <= mark {
                    hashes.push_back((String::new(), 0));
                }
                print!("reset {}from {}\n", refspec, hash);
                *hashes.get_mut(mark).unwrap() = (hash, lucky);
                state = State::RefSelected;
            }
            _ => match line.split_once(' ') {
                Some(("mark", id)) => match state {
                    State::Committing | State::Tagging => {
                        mark = id[1..id.len() - 1].parse::<usize>().unwrap();
                    }
                    _ => panic!("Illegal command mark in {:?} state", state),
                },
                Some(("original-oid", id)) => match state {
                    State::Committing => {
                        original = id[..id.len() - 1].to_string();
                        let obj = original.clone() + "^{tree}";
                        tree = std::str::from_utf8(&spawn_git(
                            &["rev-parse", "-q", obj.as_str()],
                            None,
                        ))
                        .unwrap()
                        .to_string();
                    }
                    State::Tagging => original = id[..id.len() - 1].to_string(),
                    _ => panic!("Illegal command original-oid in {:?} state", state),
                },
                Some(("author", _)) => match state {
                    State::Committing => author = line,
                    _ => panic!("Illegal command author in {:?} state", state),
                },
                Some(("committer", _)) => match state {
                    State::Committing => committer = line,
                    _ => panic!("Illegal command committer in {:?} state", state),
                },
                Some(("tagger", _)) => match state {
                    State::Tagging => committer = line,
                    _ => panic!("Illegal command tagger in {:?} state", state),
                },
                Some(("from", id)) => {
                    let p = id[1..id.len() - 1].parse::<usize>().unwrap();
                    let (h, l) = hashes.get(p).unwrap();
                    let mut invoke = |s, v| {
                        parents.push_str(s);
                        lucky = v;
                        parents.push_str(h);
                        parents.push('\n');
                    };
                    match state {
                        State::Committing => invoke("parent ", *l + 1),
                        State::Tagging => invoke("object ", *l),
                        State::RefSelected => {
                            println!("reset {}from {}", refspec, h);
                            state = State::Frommed;
                        }
                        _ => panic!("Illegal command from in {:?} state", state),
                    };
                }
                Some(("merge", id)) => match state {
                    State::Committing => {
                        let p = id[1..id.len() - 1].parse::<usize>().unwrap();
                        parents.push_str("parent ");
                        let (h, _) = hashes.get(p).unwrap();
                        parents.push_str(h);
                        parents.push('\n');
                    }
                    _ => panic!("Illegal command merge in {:?} state", state),
                },
                Some(("commit", _)) => match state {
                    State::RefSelected => {
                        state = State::Committing;
                        tree.clear();
                        lucky = 0;
                        original.clear();
                        parents.clear();
                        author.clear();
                        committer.clear();
                        message.clear();
                        mark = 0;
                    }
                    _ => panic!("Illegal command commit in {:?} state", state),
                },
                Some(("tag", _)) => match state {
                    State::RefSelected => {
                        state = State::Tagging;
                        original.clear();
                        parents.clear();
                        author = line;
                        committer.clear();
                        message.clear();
                        mark = 0;
                    }
                    _ => panic!("Illegal command tag in {:?} state", state),
                },
                Some(("data", l)) => match state {
                    State::Committing | State::Tagging => {
                        let len = l[..l.len() - 1].parse::<usize>().unwrap();
                        message = Vec::with_capacity(len);
                        unsafe {
                            message.set_len(len);
                        }
                        reader.read_exact(&mut message)?;
                    }
                    _ => panic!("Illegal command data in {:?} state", state),
                },
                Some(("reset", refs)) => match state {
                    State::Idle | State::RefSelected => {
                        state = State::RefSelected;
                        refspec = refs.to_string();
                    }
                    _ => panic!("Illegal command reset in {:?} state", state),
                },
                Some(("R", _)) | Some(("C", _)) | Some(("D", _)) | Some(("M", _))
                | Some(("N", _)) => (),
                _ => panic!("Unknown command {}", line),
            },
        }
    }

    Ok(())
}

fn spawn_git_silent(args: &[&str], stdin: Option<&[u8]>) -> Option<Vec<u8>> {
    let mut child = Command::new("git")
        .args(args)
        .stdin(if stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();
    if let Some(input) = stdin {
        child.stdin.as_mut().unwrap().write_all(input).unwrap();
    }

    let output = child.wait_with_output().unwrap();

    if !output.status.success() {
        None
    } else {
        Some(output.stdout)
    }
}

fn spawn_git(args: &[&str], stdin: Option<&[u8]>) -> Vec<u8> {
    match spawn_git_silent(args, stdin) {
        Some(v) => v,
        None => panic!("git finished with non-zero exit code: {:?}", args),
    }
}
