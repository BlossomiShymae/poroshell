use data::RiotAPILibrary;

#[derive(Debug, PartialEq, Clone)]

pub enum BackgroundCmd {
    LibrariesLoad,
    LibrariesOpenLink(String),
}

#[derive(Debug, Clone)]
pub enum BackgroundCmdResult {
    LibrariesReady(Vec<RiotAPILibrary>),
}
