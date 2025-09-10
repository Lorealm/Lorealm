pub trait Patch<T> {
    fn patch(&mut self, value: Option<T>);
}

impl<T> Patch<T> for T {
    fn patch(&mut self, value: Option<T>) {
        if let Some(val) = value {
            *self = val;
        }
    }
}

impl<T> Patch<T> for Option<T> {
    fn patch(&mut self, value: Option<T>) {
        if let Some(val) = value {
            *self = Some(val);
        }
    }
}

pub fn patch_variable<T, U>(variable: &mut U, value: Option<T>)
where
    U: Patch<T>,
{
    variable.patch(value);
}