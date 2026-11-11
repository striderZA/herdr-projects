//! The host, the repo and whether it is an ssh one, for every form a clone's remote can have: what the gh gate and the
//! install lookup read from a remote's URL.
use sw::install::*;

#[test]
fn a_remote_names_its_host_its_repo_and_whether_it_is_ssh() {
    for (url, want) in [
        ("https://ghe.example.com:8443/a/b.git", Some(("ghe.example.com", "a/b", false))),
        ("ssh://git@ghe.example.com:22/a/b", Some(("ghe.example.com", "a/b", true))),
        ("git@github-work:striderZA/herdr-wingman.git", Some(("github-work", "striderZA/herdr-wingman", true))),
        ("https://user:x@GHE.example.com/a/b/", Some(("ghe.example.com", "a/b", false))),
        ("C:/a/b", None),
        ("/local/path/origin.git", None),
        ("file:///tmp/origin.git", None),
        ("https://ghe.example.com/a/b/c", None),
    ] {
        let got = remote_of(url).map(|r| (r.host, r.repo, r.ssh));
        assert_eq!(got, want.map(|(h, r, s)| (h.to_string(), r.to_string(), s)), "{url:?}");
    }
}
