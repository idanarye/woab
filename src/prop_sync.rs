use gtk4::prelude::*;

/// Set widgets values from some setter type. See [`#[derive(woab::PropSync)]`](crate::PropSync).
pub trait SetProps<'a> {
    /// The type of the setter.
    ///
    /// Usually a Rust primitive that implements `glib::value::ToValue` for widgets, or a generated
    /// struct when generated from [`#[derive(woab::PropSync)]`](crate::PropSync).
    type SetterType: 'a;

    /// Set the widgets' data from the setter type.
    fn set_props(&self, setter: &Self::SetterType);
}

/// Get widgets values into some getter type. See [`#[derive(woab::PropSync)]`](crate::PropSync).
pub trait GetProps {
    /// The type of the getter.
    ///
    /// Usually a Rust primitive that implements `glib::value::FromValueOptional` for widgets, or a
    /// generated struct when generated from [`#[derive(woab::PropSync)]`](crate::PropSync).
    type GetterType;

    /// Get the widgets' data into the getter type.
    fn get_props(&self) -> Self::GetterType;
}

impl<'a> SetProps<'a> for gtk4::Label {
    type SetterType = &'a str;

    fn set_props(&self, setter: &Self::SetterType) {
        self.set_text(setter);
    }
}

impl<'a> SetProps<'a> for gtk4::Entry {
    type SetterType = &'a str;

    fn set_props(&self, setter: &Self::SetterType) {
        self.set_text(setter);
    }
}

impl GetProps for gtk4::Entry {
    type GetterType = String;

    fn get_props(&self) -> Self::GetterType {
        self.text().to_string()
    }
}

impl<'a> SetProps<'a> for gtk4::CheckButton {
    type SetterType = bool;

    fn set_props(&self, setter: &Self::SetterType) {
        self.set_active(*setter);
    }
}

impl GetProps for gtk4::CheckButton {
    type GetterType = bool;

    fn get_props(&self) -> Self::GetterType {
        self.is_active()
    }
}
