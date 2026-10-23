//! Deliberately supplied run-instance to PR inputs, held only for this launch. Prose, slugs and
//! branches cannot establish a result link; conflicting declarations remain unknown, never last-wins.

use std::collections::BTreeMap;

/// Explicit result links for one launch, not a mapping store or approval record.
#[derive(Clone, Debug, Default)]
pub struct ResultLinks {
    links: BTreeMap<String, Option<String>>,
}

impl ResultLinks {
    /// Read one RUN_ID=PR_URL declaration. Malformed identities refuse configuration; conflicting
    /// declarations for the same run remain unknown. This never reads files, contacts GitHub or writes.
    pub fn add(&mut self, declaration: &str) -> Result<(), String> {
        let (id, url) = declaration
            .split_once('=')
            .ok_or("--taste-result needs RUN_ID=https://github.com/OWNER/REPO/pull/NUMBER")?;
        if !canonical_run_id(id) {
            return Err("--taste-result needs a canonical lowercase UUIDv4 run ID".into());
        }
        let pattern = regex::Regex::new(
            r"^https://github\.com/[A-Za-z0-9-]+/[A-Za-z0-9_.-]+/pull/[1-9][0-9]*$",
        )
        .unwrap();
        if !pattern.is_match(url)
            || url
                .split('/')
                .nth(4)
                .is_some_and(|repo| matches!(repo, "." | ".."))
        {
            return Err("--taste-result needs a full canonical https://github.com/OWNER/REPO/pull/NUMBER URL".into());
        }
        self.links
            .entry(id.into())
            .and_modify(|existing| {
                if existing.as_deref() != Some(url) {
                    *existing = None;
                }
            })
            .or_insert_with(|| Some(url.into()));
        Ok(())
    }

    /// Only the exact source-owned run ID can select an explicit result; conflicts never recover
    /// by repeated declarations. A new launch is required to supply corrected inputs.
    pub(super) fn url(&self, id: &str) -> Result<String, String> {
        match self.links.get(id) {
            Some(Some(url)) => Ok(url.clone()),
            Some(None) => Err(
                "Conflicting --taste-result inputs for this run; relaunch with one PR URL.".into(),
            ),
            None => Err(format!(
                "Result unknown: relaunch with --taste-result {id}=PR_URL."
            )),
        }
    }
}

/// Source run IDs and launch inputs must use the same canonical UUIDv4 spelling.
pub(super) fn canonical_run_id(id: &str) -> bool {
    regex::Regex::new(r"^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$")
        .unwrap()
        .is_match(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    const ID: &str = "00000000-0000-4000-8000-000000000114";
    const OTHER: &str = "00000000-0000-4000-8000-000000000115";
    const URL: &str = "https://github.com/TEST/clinic-tycoon/pull/1";

    #[test]
    fn exact_run_ids_only_duplicates_agree_and_conflicts_stay_unknown() {
        let mut links = ResultLinks::default();
        assert!(links.url(ID).is_err());
        links.add(&format!("{ID}={URL}")).unwrap();
        links.add(&format!("{ID}={URL}")).unwrap();
        assert_eq!(links.url(ID).unwrap(), URL);
        assert!(
            links.url(OTHER).is_err(),
            "a different run must never borrow this PR"
        );
        links
            .add(&format!(
                "{ID}=https://github.com/TEST/clinic-tycoon/pull/2"
            ))
            .unwrap();
        links.add(&format!("{ID}={URL}")).unwrap();
        assert!(links.url(ID).unwrap_err().contains("Conflicting"));
    }

    #[test]
    fn malformed_ids_urls_and_noncanonical_shortcuts_refuse_before_any_read() {
        for id in [
            "TEST-result",
            "",
            "00000000-0000-1000-8000-000000000114",
            "00000000-0000-4000-7000-000000000114",
            "AAAAAAAA-0000-4000-8000-000000000114",
        ] {
            assert!(
                ResultLinks::default().add(&format!("{id}={URL}")).is_err(),
                "{id}"
            );
        }
        for url in [
            "TEST/clinic-tycoon#1",
            "http://github.com/TEST/clinic-tycoon/pull/1",
            "https://evil.test/TEST/clinic-tycoon/pull/1",
            "https://github.com/TEST/clinic-tycoon/pull/0",
            "https://github.com/TEST/clinic-tycoon/pull/1/",
            "https://github.com/TEST/clinic-tycoon/pull/1?x=1",
            "https://github.com/TEST/clinic-tycoon/pull/1#review",
            "https://github.com/TEST/../pull/1",
            "https://github.com/TEST/clinic-tycoon/pull/1\n",
        ] {
            assert!(
                ResultLinks::default().add(&format!("{ID}={url}")).is_err(),
                "{url}"
            );
        }
        assert!(ResultLinks::default().add(ID).is_err());
    }
}
