//! Inference settings that belong to one model, not to the program:
//!   * `frc_ignore`: the fr-check lints the model reads as inputs (`frc=<RULE>` atoms) are those whose upper-cased rule
//!     name contains none of these strings (natural.py `IGN`). It must be the list the model was trained with: a model
//!     trained with the capitalisation lints as inputs scores wrongly without them, and the other way round.
//!   * `threshold`: the default score above which the model flags a word.
//!
//! Where they come from, in order:
//!   1. the model header's `infer` block, `{"name": "...", "frc_ignore": ["SPELL", ...], "threshold": 0.99}`, for
//!      models exported with it;
//!   2. the table below, for models exported before that block existed, keyed by the sha256 of the training checkpoint
//!      that the header records (`source_sha256`, written by training/export_ebm.py);
//!   3. otherwise loading fails: an unknown model never runs with settings it was not made for.
use serde_json::Value;

#[derive(Clone, Debug, PartialEq)]
pub struct Profile {
    /// model name, for messages
    pub name: String,
    /// upper-case substrings of fr-check rule names whose lints are not model inputs
    pub frc_ignore: Vec<String>,
    /// default threshold (0..1; a word is flagged when its score is above it)
    pub threshold: f64,
}

struct Known { source_sha256: &'static str, name: &'static str, frc_ignore: &'static [&'static str], threshold: f64 }

/// The v15 lint list: every lint except spelling, typography, spacing and duplicated words. H1 was trained with the
/// capitalisation lints (FR_CAPITAL) as inputs; v14 also ignored CAPITAL and MAJ.
pub const IGN_V15: [&str; 6] = ["SPELL", "TYPO", "SPACE", "NBSP", "DUP", "DOUBLE"];

const KNOWN: [Known; 1] = [
    // H1 = h1_hom_warm8k (owner decision 2026-11-09): threshold 0.9914 = as many flags on clean book text as S1 at its
    // own threshold (3.5 per 1,000 sentences); see the research repository, models/h1/MODEL_CARD.md.
    Known {
        source_sha256: "2700697db69b3f6018096d5438bbf712686f9a0b4bdf34858f0cd1246991e181",
        name: "H1 (h1_hom_warm8k)",
        frc_ignore: &IGN_V15,
        threshold: 0.9914,
    },
];

impl Profile {
    /// The profile of the model whose header is `head` (see the module documentation).
    pub fn from_header(head: &Value) -> Result<Profile, String> {
        if let Some(b) = head.get("infer") {
            let name = b.get("name").and_then(Value::as_str).ok_or("model header: infer.name missing")?.to_string();
            let frc_ignore = b.get("frc_ignore").and_then(Value::as_array).ok_or("model header: infer.frc_ignore missing")?
                .iter().map(|x| x.as_str().map(str::to_uppercase).ok_or_else(|| "model header: infer.frc_ignore holds a non-string".to_string()))
                .collect::<Result<Vec<_>, _>>()?;
            let threshold = b.get("threshold").and_then(Value::as_f64).ok_or("model header: infer.threshold missing")?;
            if !(0.0..=1.0).contains(&threshold) { return Err(format!("model header: infer.threshold {threshold} is not between 0 and 1")); }
            return Ok(Profile { name, frc_ignore, threshold });
        }
        let sha = head.get("source_sha256").and_then(Value::as_str).unwrap_or("");
        match KNOWN.iter().find(|k| k.source_sha256 == sha) {
            Some(k) => Ok(Profile { name: k.name.to_string(), frc_ignore: k.frc_ignore.iter().map(|s| s.to_string()).collect(), threshold: k.threshold }),
            None => Err(format!("unknown model (checkpoint sha256 {}): its header has no `infer` block giving the fr-check lints it \
                                 reads and its threshold, and it is not one of the models Vaugelas knows",
                                if sha.is_empty() { "not recorded" } else { sha })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn known_model_by_checkpoint() {
        let p = Profile::from_header(&json!({"source_sha256": KNOWN[0].source_sha256})).unwrap();
        assert_eq!(p.frc_ignore, ["SPELL", "TYPO", "SPACE", "NBSP", "DUP", "DOUBLE"]);
        assert_eq!(p.threshold, 0.9914);
    }

    #[test]
    fn header_block_wins() {
        let h = json!({"source_sha256": KNOWN[0].source_sha256, "infer": {"name": "x", "frc_ignore": ["spell", "Capital"], "threshold": 0.5}});
        let p = Profile::from_header(&h).unwrap();
        assert_eq!((p.name.as_str(), p.frc_ignore.clone(), p.threshold), ("x", vec!["SPELL".to_string(), "CAPITAL".to_string()], 0.5));
        assert!(Profile::from_header(&json!({"infer": {"name": "x", "frc_ignore": [], "threshold": 2.0}})).is_err());
        assert!(Profile::from_header(&json!({"infer": {"name": "x", "threshold": 0.5}})).is_err());
    }

    #[test]
    fn unknown_model_is_refused() {
        let e = Profile::from_header(&json!({"source_sha256": "00"})).unwrap_err();
        assert!(e.contains("unknown model"), "{e}");
        assert!(Profile::from_header(&json!({})).is_err());
    }
}
