#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Msg {
    AppClose,
    LibrariesInit,
    LibrariesSubmit(usize),
    QuitDialogShow,
    QuitDialogCancel,
    QuitDialogOk,
    None,
}
