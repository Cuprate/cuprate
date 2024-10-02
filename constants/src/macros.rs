/// Output a string link to `monerod` source code.
#[allow(
    clippy::allow_attributes,
    unused_macros,
    reason = "used in feature gated modules"
)]
macro_rules! monero_definition_link {
    (
        $commit:ident, // Git commit hash
        $file_path:literal, // File path within `monerod`'s `src/`, e.g. `rpc/core_rpc_server_commands_defs.h`
        $start:literal$(..=$end:literal)? // File lines, e.g. `0..=123` or `0`
    ) => {
        concat!(
            "",
            "[Original definition](https://github.com/monero-project/monero/blob/",
            stringify!($commit),
            "/src/",
            $file_path,
            "#L",
            stringify!($start),
            $(
                "-L",
                stringify!($end),
            )?
            ")."
        )
    };
}

#[allow(
    clippy::allow_attributes,
    unused_imports,
    reason = "used in feature gated modules"
)]
pub(crate) use monero_definition_link;
