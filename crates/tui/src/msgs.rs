#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Msg {
    AppClose,
    LibrariesInit,
    LibrariesSubmit(usize),
    LibrariesBlur,
    NavigationBlur,
    QuitDialogShow,
    QuitDialogCancel,
    QuitDialogOk,
    None,
}
