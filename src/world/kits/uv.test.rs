use super::{PINS, checked};

#[test]
fn an_archive_whose_sha256_is_not_the_pin_is_refused_with_how_to_install_uv_by_hand() {
  let url =
    "https://github.com/astral-sh/uv/releases/download/0.13.1/uv-aarch64-apple-darwin.tar.gz";
  let refused = checked(&PINS[0], url.to_owned(), b"not uv").map_err(|no| no.to_string());
  assert_eq!(
    refused,
    Err(format!(
      "the archive of {url} has the sha256 \
       06b1ac1bb374e22190983c338745cfb497f07668e8e8728d47c8d1734bf6f46a, not the one that furb \
       pins. Install uv as https://docs.astral.sh/uv/getting-started/installation/ says, or name \
       its program in FURB_UV."
    ))
  );
}
