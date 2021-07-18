#[macro_export]
macro_rules! make_as_trait {
    ($traitname:ident) => {
        paste::paste! {
            trait [<Has $traitname>] : $traitname { }
            trait [<As $traitname>] {
                fn [<as_ $traitname:snake>](&self) -> Option<&dyn $traitname> {
                    None
                }
                fn [<as_ $traitname:snake _mut>](&mut self) -> Option<&mut dyn $traitname> {
                    None
                }
            }
            impl<T: [<Has $traitname>]> [<As $traitname>] for T {
                fn [<as_ $traitname:snake>](&self) -> Option<&dyn $traitname> {
                    Some(self)
                }
                fn [<as_ $traitname:snake _mut>](&mut self) -> Option<&mut dyn $traitname> {
                    Some(self)
                }
            }
        }
    }
}
