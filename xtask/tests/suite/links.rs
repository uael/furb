//! The task `links`, on a repository in a yard whose package carries its README.

use std::{fs, process};

use crate::{ended, held, xtask, yard::Yard};

#[test]
fn a_relative_link_of_a_readme_of_a_package_fails_until_the_fix_writes_it_absolute() {
  let yard = Yard::new(format!("xtask-links-{}", process::id()));
  let readme = "\
[guide](pages/guide.md#start) and ![logo](pages/logo.png) and [lost](pages/lost.md)
`[code](pages/guide.md)`

```
[fence](pages/guide.md)
```

[ref]: pages
<img src=\"pages/logo.png\"> [gone](https://github.com/uael/furb/blob/main/pages/gone.md) [web](https://example.com)
";
  let files = [
    ("README.md", readme),
    ("package.json", "{\"name\": \"pkg\"}"),
    ("pages/guide.md", ""),
    ("pages/logo.png", ""),
  ];
  held(&yard, &files);
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
  let carried = "pkg (package.json) carries README.md\n".to_owned();
  let failed = |lines: &[&String]| {
    let lines: Vec<&str> = lines.iter().map(|one| one.as_str()).collect();
    (
      Some(1),
      carried.clone(),
      format!(
        "{}\ncargo xtask links --fix writes each relative link as absolute.\n",
        lines.join("\n")
      ),
    )
  };
  assert_eq!(ended(&mut xtask(&yard, ["links"])), failed(&wrong.iter().collect::<Vec<_>>()));
  let left = failed(&[&wrong[2], &wrong[5]]);
  assert_eq!(
    ended(&mut xtask(&yard, ["links", "--fix"])),
    left,
    "the fix names what it cannot fix"
  );
  assert_eq!(
    ended(&mut xtask(&yard, ["links"])),
    left,
    "the fix writes each relative link that points at a file, and no link in code"
  );
  let text = fs::read_to_string(yard.join("README.md")).unwrap_or_default();
  assert!(
    text.contains("`[code](pages/guide.md)`") && text.contains("[fence](pages/guide.md)"),
    "{text}"
  );
}
