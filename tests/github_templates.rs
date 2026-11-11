//! The GitHub templates: the pull request template names no issue to close, so the board's `Closes #N` lines are the only ones.
use sw::land::closes::with_lines;

/// The pull request template as GitHub opens it. A `Closes #N` written in it, even in one of its comments (a pull request
/// keeps those), would count as an issue the body already closes, and the board would then leave that line out.
#[test]
fn the_board_writes_each_closes_line_it_wants_under_the_pull_request_template() {
    let template = include_str!("../.github/PULL_REQUEST_TEMPLATE.md");
    let (body, added) = with_lines(template, &[12, 13]);
    assert_eq!(added, vec![12, 13], "the template already closes an issue: {body}");
    assert!(body.ends_with("\n\nCloses #12\nCloses #13"), "the lines go at the end, one per issue: {body}");
}
