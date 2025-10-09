//! Offline stub for `svm-rs-builds` providing deterministic release metadata without network calls.

use const_hex::decode;
use semver::Version;

pub const TARGET_PLATFORM: &str = "linux-aarch64";

pub const SOLC_VERSION_0_4_9: Version = Version::new(0, 4, 9);
pub const SOLC_VERSION_0_4_9_CHECKSUM: &str =
    "71da154585e0c9048445b39b3662b421d20814cc68482b6b072aae2e541a4c74";
pub const SOLC_VERSION_0_4_8: Version = Version::new(0, 4, 8);
pub const SOLC_VERSION_0_4_8_CHECKSUM: &str =
    "ee76039f933938cb5c14bf3fc4754776aa3e5c4c88420413da4c0c13731b8ffe";
pub const SOLC_VERSION_0_4_7: Version = Version::new(0, 4, 7);
pub const SOLC_VERSION_0_4_7_CHECKSUM: &str =
    "e1affb6e13dee7b14039f8eb1a52343f5fdc56169023e0f7fc339dfc25ad2b3d";
pub const SOLC_VERSION_0_4_6: Version = Version::new(0, 4, 6);
pub const SOLC_VERSION_0_4_6_CHECKSUM: &str =
    "0525d7b95549db6c913edb3c1b0c26d2db81e64b03f8352261df1b2ad696a65e";
pub const SOLC_VERSION_0_4_5: Version = Version::new(0, 4, 5);
pub const SOLC_VERSION_0_4_5_CHECKSUM: &str =
    "6f46ab7747d7de1b75907e539e6e19be201680e64ce99b583c6356e4e7897406";
pub const SOLC_VERSION_0_4_4: Version = Version::new(0, 4, 4);
pub const SOLC_VERSION_0_4_4_CHECKSUM: &str =
    "25d148e9c1052631a930bfbe8e4e3d9dae8de7659f8d3ea659a3ef139cd5e2c9";
pub const SOLC_VERSION_0_4_3: Version = Version::new(0, 4, 3);
pub const SOLC_VERSION_0_4_3_CHECKSUM: &str =
    "1dc7ef0b4aab472299e77b39c7465cd5ed4609a759b52ce1a93f2d54395da73a";
pub const SOLC_VERSION_0_4_2: Version = Version::new(0, 4, 2);
pub const SOLC_VERSION_0_4_2_CHECKSUM: &str =
    "891d0b2d3a636ff40924802a6f5beb1ecbc42d5d0d0bfecbbb148b561c861fb9";
pub const SOLC_VERSION_0_4_1: Version = Version::new(0, 4, 1);
pub const SOLC_VERSION_0_4_1_CHECKSUM: &str =
    "a0c06d0c6a14c66ddeca1f065461fb0024de89421c1809a1b103b55c94e30860";
pub const SOLC_VERSION_0_4_0: Version = Version::new(0, 4, 0);
pub const SOLC_VERSION_0_4_0_CHECKSUM: &str =
    "e26d188284763684f3cf6d4900b72f7e45a050dd2b2707320273529d033cfd47";

pub static ALL_SOLC_VERSIONS: [Version; 10] = [
    SOLC_VERSION_0_4_9,
    SOLC_VERSION_0_4_8,
    SOLC_VERSION_0_4_7,
    SOLC_VERSION_0_4_6,
    SOLC_VERSION_0_4_5,
    SOLC_VERSION_0_4_4,
    SOLC_VERSION_0_4_3,
    SOLC_VERSION_0_4_2,
    SOLC_VERSION_0_4_1,
    SOLC_VERSION_0_4_0,
];

pub fn get_checksum(version: &Version) -> Option<Vec<u8>> {
    let checksum = match (version.major, version.minor, version.patch) {
        (0, 4, 9) => SOLC_VERSION_0_4_9_CHECKSUM,
        (0, 4, 8) => SOLC_VERSION_0_4_8_CHECKSUM,
        (0, 4, 7) => SOLC_VERSION_0_4_7_CHECKSUM,
        (0, 4, 6) => SOLC_VERSION_0_4_6_CHECKSUM,
        (0, 4, 5) => SOLC_VERSION_0_4_5_CHECKSUM,
        (0, 4, 4) => SOLC_VERSION_0_4_4_CHECKSUM,
        (0, 4, 3) => SOLC_VERSION_0_4_3_CHECKSUM,
        (0, 4, 2) => SOLC_VERSION_0_4_2_CHECKSUM,
        (0, 4, 1) => SOLC_VERSION_0_4_1_CHECKSUM,
        (0, 4, 0) => SOLC_VERSION_0_4_0_CHECKSUM,
        _ => return None,
    };
    decode(checksum).ok()
}

pub static RELEASE_LIST_JSON: &str = r#"{"builds":[{"version":"0.4.9","sha256":"0x71da154585e0c9048445b39b3662b421d20814cc68482b6b072aae2e541a4c74"},{"version":"0.4.8","sha256":"0xee76039f933938cb5c14bf3fc4754776aa3e5c4c88420413da4c0c13731b8ffe"},{"version":"0.4.7","sha256":"0xe1affb6e13dee7b14039f8eb1a52343f5fdc56169023e0f7fc339dfc25ad2b3d"},{"version":"0.4.6","sha256":"0x0525d7b95549db6c913edb3c1b0c26d2db81e64b03f8352261df1b2ad696a65e"},{"version":"0.4.5","sha256":"0x6f46ab7747d7de1b75907e539e6e19be201680e64ce99b583c6356e4e7897406"},{"version":"0.4.4","sha256":"0x25d148e9c1052631a930bfbe8e4e3d9dae8de7659f8d3ea659a3ef139cd5e2c9"},{"version":"0.4.3","sha256":"0x1dc7ef0b4aab472299e77b39c7465cd5ed4609a759b52ce1a93f2d54395da73a"},{"version":"0.4.2","sha256":"0x891d0b2d3a636ff40924802a6f5beb1ecbc42d5d0d0bfecbbb148b561c861fb9"},{"version":"0.4.1","sha256":"0xa0c06d0c6a14c66ddeca1f065461fb0024de89421c1809a1b103b55c94e30860"},{"version":"0.4.0","sha256":"0xe26d188284763684f3cf6d4900b72f7e45a050dd2b2707320273529d033cfd47"}],"releases":{"0.4.0":"solc-v0.4.0","0.4.1":"solc-v0.4.1","0.4.2":"solc-v0.4.2","0.4.3":"solc-v0.4.3","0.4.4":"solc-v0.4.4","0.4.5":"solc-v0.4.5","0.4.6":"solc-v0.4.6","0.4.7":"solc-v0.4.7","0.4.8":"solc-v0.4.8","0.4.9":"solc-v0.4.9"}}"#;
