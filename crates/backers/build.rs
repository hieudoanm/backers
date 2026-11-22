//! Embeds the build timestamp so `backers --version` can report it.

fn main() {
    let now = chrono::Utc::now();
    println!(
        "cargo:rustc-env=BACKERS_BUILD_TIME={}",
        now.format("%Y.%m.%d.%H.%M.%S")
    );
}
