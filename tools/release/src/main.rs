//! The release tool (Phase 5; runbook `docs/release/signing.md`): data
//! keys, detached signatures, and (P4) signed bundle packs and the data
//! feed. Maintainer- and release-workflow-run; the application only
//! verifies (`loom_data::signature`).
//!
//! A secret key is 64 hex digits (the Ed25519 seed). It is read from a
//! file (`--secret <file>`) or from an environment variable
//! (`--secret-env <NAME>`, the release workflow's secret), never from an
//! argument, so it stays out of process listings and shell history.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use loom_data::signature::{self, KeyPair, Signature, TrustList};

const USAGE: &str = "\
usage:
  loom-release keygen --out <dir> [--note <text>]
      a new data key from the OS's random source: <dir>/<key id>.secret
      (keep it offline) and its trust list entry, printed
  loom-release trust (--secret <file> | --secret-env <NAME>) [--note <text>]
      prints the trust list entry of a secret key
  loom-release trust-add <trust.json> (--secret <file> | --secret-env <NAME>) [--note <text>]
      adds the key to a trust list file (the release workflow's throwaway mode)
  loom-release sign <file> (--secret <file> | --secret-env <NAME>) --out <signature.json>
      a detached signature over the file's bytes
  loom-release test-trust
      prints release/trust-test.json (the public test key's trust list)
  loom-release test-sign <file> --out <signature.json>
      a signature with the public test key (fixtures; only debug and e2e builds trust it)
  loom-release pack <bundle dir> (--secret <file> | --secret-env <NAME> | --test-key | --unsigned) --out <x.loombundle>
      a data bundle pack: bundle.json, its signature, every listed file
  loom-release feed --sequence <n> --base-url <https://.../download/vX> --out <data-feed.json> <pack>...
      the data feed listing the packs (sign it with `sign`; the sequence must grow)
  loom-release verify <file> <signature.json> --trust <trust.json>
      exit 0 when the signature verifies with a trusted, unrevoked key";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("loom-release: {e}");
            ExitCode::FAILURE
        }
    }
}

/// The value after `--name`, if given.
fn flag<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .map(String::as_str)
}

/// The positional arguments (not flags or their values).
fn positional(args: &[String]) -> Vec<&str> {
    let mut out = Vec::new();
    let mut skip = false;
    for a in args {
        if skip {
            skip = false;
        } else if a.starts_with("--") {
            // Every flag takes a value but these.
            skip = !matches!(a.as_str(), "--test-key" | "--unsigned");
        } else {
            out.push(a.as_str());
        }
    }
    out
}

/// The secret key from `--secret <file>` or `--secret-env <NAME>`.
fn secret(args: &[String]) -> Result<KeyPair, String> {
    let text = if let Some(file) = flag(args, "--secret") {
        std::fs::read_to_string(file).map_err(|e| format!("{file}: {e}"))?
    } else if let Some(name) = flag(args, "--secret-env") {
        std::env::var(name).map_err(|_| format!("environment variable {name} is not set"))?
    } else {
        return Err("give --secret <file> or --secret-env <NAME>".to_owned());
    };
    KeyPair::from_hex(&text).map_err(|e| e.to_string())
}

fn read(path: &str) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|e| format!("{path}: {e}"))
}

fn write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    std::fs::write(path, bytes).map_err(|e| format!("{}: {e}", path.display()))
}

fn json<T: serde::Serialize>(value: &T) -> Result<String, String> {
    serde_json::to_string_pretty(value)
        .map(|s| s + "\n")
        .map_err(|e| e.to_string())
}

fn run(args: &[String]) -> Result<(), String> {
    let (command, rest) = args.split_first().ok_or(USAGE)?;
    let note = flag(rest, "--note").unwrap_or_default();
    match command.as_str() {
        "keygen" => {
            let out = PathBuf::from(flag(rest, "--out").ok_or(USAGE)?);
            let mut seed = [0u8; 32];
            getrandom::fill(&mut seed).map_err(|e| format!("random source: {e}"))?;
            let pair = KeyPair::from_seed(&seed);
            let entry = pair.trusted(note);
            std::fs::create_dir_all(&out).map_err(|e| format!("{}: {e}", out.display()))?;
            let file = out.join(format!("{}.secret", entry.id.replace(':', "-")));
            if file.exists() {
                return Err(format!("{} exists", file.display()));
            }
            write(&file, (signature::hex(&seed) + "\n").as_bytes())?;
            eprintln!("secret key: {} (keep it offline)", file.display());
            print!("{}", json(&entry)?);
            Ok(())
        }
        "trust" => {
            print!("{}", json(&secret(rest)?.trusted(note))?);
            Ok(())
        }
        "test-trust" => {
            let pair = KeyPair::test_key();
            let list = TrustList {
                format: signature::TRUST_FORMAT,
                keys: vec![pair.trusted(
                    "The public test key (seed: SHA-256 of \"loom-test-data-key\"). Trusted only by debug and e2e builds.",
                )],
                revoked: Vec::new(),
            };
            print!("{}", json(&list)?);
            Ok(())
        }
        "test-sign" => {
            let [file] = positional(rest)[..] else {
                return Err(USAGE.to_owned());
            };
            let out = flag(rest, "--out").ok_or(USAGE)?;
            let sig = KeyPair::test_key().sign(&read(file)?);
            write(Path::new(out), json(&sig)?.as_bytes())
        }
        "trust-add" => {
            let [file] = positional(rest)[..] else {
                return Err(USAGE.to_owned());
            };
            let list = TrustList::parse(&read(file)?).map_err(|e| format!("{file}: {e}"))?;
            let entry = secret(rest)?.trusted(note);
            let list = list.with(&TrustList {
                format: signature::TRUST_FORMAT,
                keys: vec![entry],
                revoked: Vec::new(),
            });
            write(Path::new(file), json(&list)?.as_bytes())
        }
        "sign" => {
            let [file] = positional(rest)[..] else {
                return Err(USAGE.to_owned());
            };
            let out = flag(rest, "--out").ok_or(USAGE)?;
            let sig = secret(rest)?.sign(&read(file)?);
            write(Path::new(out), json(&sig)?.as_bytes())
        }
        "pack" => {
            let [dir] = positional(rest)[..] else {
                return Err(USAGE.to_owned());
            };
            let out = flag(rest, "--out").ok_or(USAGE)?;
            let manifest = read(&format!("{dir}/bundle.json"))?;
            let pair = if rest.iter().any(|a| a == "--unsigned") {
                None
            } else if rest.iter().any(|a| a == "--test-key") {
                Some(KeyPair::test_key())
            } else {
                Some(secret(rest)?)
            };
            let sig = pair
                .map(|p| serde_json::to_vec_pretty(&p.sign(&manifest)))
                .transpose()
                .map_err(|e| e.to_string())?;
            loom_app::install::write_pack(Path::new(dir), sig.as_deref(), Path::new(out))
                .map_err(|e| e.to_string())?;
            println!("pack: {out} ({})", pack_id(out)?);
            Ok(())
        }
        "feed" => {
            let sequence: u64 = flag(rest, "--sequence")
                .ok_or(USAGE)?
                .parse()
                .map_err(|_| "--sequence: a whole number".to_owned())?;
            let base = flag(rest, "--base-url").ok_or(USAGE)?.trim_end_matches('/');
            if !base.starts_with("https://") {
                return Err("--base-url must be https".to_owned());
            }
            let out = flag(rest, "--out").ok_or(USAGE)?;
            let mut bundles = Vec::new();
            for pack in positional(rest) {
                let bytes = read(pack)?;
                let name = Path::new(pack)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .ok_or_else(|| format!("{pack}: no file name"))?;
                bundles.push(loom_data::pack::FeedBundle {
                    id: pack_id(pack)?,
                    url: format!("{base}/{name}"),
                    sha256: signature::hex(&sha2::Digest::finalize(
                        <sha2::Sha256 as sha2::Digest>::new_with_prefix(&bytes),
                    )),
                    size: bytes.len() as u64,
                });
            }
            let feed = loom_data::pack::Feed {
                format: loom_data::pack::FEED_FORMAT,
                sequence,
                bundles,
            };
            write(Path::new(out), json(&feed)?.as_bytes())
        }
        "verify" => {
            let [file, sig] = positional(rest)[..] else {
                return Err(USAGE.to_owned());
            };
            let trust_file = flag(rest, "--trust").ok_or(USAGE)?;
            let trust =
                TrustList::parse(&read(trust_file)?).map_err(|e| format!("{trust_file}: {e}"))?;
            let sig: Signature =
                serde_json::from_slice(&read(sig)?).map_err(|e| format!("{sig}: {e}"))?;
            let key = signature::verify(&read(file)?, &sig, &trust).map_err(|e| e.to_string())?;
            println!("verified: {} ({})", key.id, key.note);
            Ok(())
        }
        _ => Err(USAGE.to_owned()),
    }
}

/// The bundle ID a pack's manifest names.
fn pack_id(pack: &str) -> Result<String, String> {
    use std::io::Read as _;
    let file = std::fs::File::open(pack).map_err(|e| format!("{pack}: {e}"))?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| format!("{pack}: {e}"))?;
    let mut bytes = Vec::new();
    zip.by_name(loom_data::bundle::MANIFEST)
        .map_err(|e| format!("{pack}: {e}"))?
        .take(loom_data::pack::MAX_SMALL_FILE)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("{pack}: {e}"))?;
    let manifest = loom_data::pack::manifest(&bytes).map_err(|e| format!("{pack}: {e}"))?;
    let game = manifest
        .game()
        .ok_or_else(|| format!("{pack}: unknown game"))?;
    Ok(loom_app::bundles::bundle_id(&loom_domain::DataVersion {
        game,
        schema_version: manifest.data_schema,
        steam_build: manifest.source.steam_build,
    }))
}
