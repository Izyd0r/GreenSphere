fn main() {
    if let Ok(it) = dotenvy::dotenv_iter() {
        for item in it {
            if let Ok((key, value)) = item {
                if key == "FIREBASE_URL" {
                    println!("cargo:rustc-env={}={}", key, value);
                }
            }
        }
    }
    
    println!("cargo:rerun-if-changed=.env");
}