//! The tests of the links of the READMEs of the packages, on a repository in a yard.

use std::{fs, process::id};

use super::checked;
use crate::yard::Yard;

#[test]
fn a_relative_link_of_a_readme_of_a_package_fails_until_the_fix_writes_it_absolute() {
  let yard = Yard::new(format!("xtask-links-{}", id()));
  let readme = "\
[guide](pages/guide.md#start) and ![logo](pages/logo.png) and [lost](pages/lost.md)
`[code](pages/guide.md)`

```
[fence](pages/guide.md)
```

[ref]: pages
<img src=\"pages/logo.png\"> [gone](https://github.com/uael/furb/blob/main/pages/gone.md) [web](https://example.com)
";
  fs::create_dir_all(yard.join("pages")).unwrap();
  fs::write(yard.join("README.md"), readme).unwrap();
  let files = ["README.md", "pages/guide.md", "pages/logo.png"].into_iter().collect();
  let blob = "https://github.com/uael/furb/blob/main";
  let raw = "https://raw.githubusercontent.com/uael/furb/main";
  let relative = "is relative, so it breaks on a registry: write";
  let wrong = [
    format!("README.md:1: pages/guide.md#start {relative} {blob}/pages/guide.md#start"),
    format!("README.md:1: pages/logo.png {relative} {raw}/pages/logo.png"),
    "README.md:1: pages/lost.md is relative and points at no file of the repository".to_owned(),
    format!("README.md:8: pages {relative} https://github.com/uael/furb/tree/main/pages"),
    format!("README.md:9: pages/logo.png {relative} {raw}/pages/logo.png"),
    format!("README.md:9: {blob}/pages/gone.md points at no file of the repository"),
  ];
  assert_eq!(checked(&yard, "README.md", &files, false).unwrap(), wrong);
  checked(&yard, "README.md", &files, true).unwrap();
  assert_eq!(
    checked(&yard, "README.md", &files, false).unwrap(),
    [wrong[2].clone(), wrong[5].clone()],
    "the fix writes each relative link that points at a file, and no link in code"
  );
  let fixed = fs::read_to_string(yard.join("README.md")).unwrap();
  assert!(
    fixed.contains("`[code](pages/guide.md)`") && fixed.contains("[fence](pages/guide.md)"),
    "{fixed}"
  );
}
