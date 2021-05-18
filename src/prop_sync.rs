use gtk::prelude::*;

pub trait SetProps<'a> {
    type SetterType: 'a;

    fn set_props(&self, setter: &Self::SetterType);
}

pub trait GetProps {
    type GetterType;

    fn get_props(&self) -> Self::GetterType;
}

impl<'a> SetProps<'a> for gtk::Entry {
    type SetterType = &'a str;

    fn set_props(&self, setter: &Self::SetterType) {
        self.set_text(setter);
    }
}

impl GetProps for gtk::Entry {
    type GetterType = String;

    fn get_props(&self) -> Self::GetterType {
        self.get_text().to_string()
    }
}
