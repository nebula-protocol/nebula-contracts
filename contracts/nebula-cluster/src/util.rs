/// ## Description
/// Prints a vector in a pretty format.
///
/// ## Params
/// - **v** is an object of type [`&Vec<T>`] where `T` is any types implementing `ToString`.
pub fn vec_to_string<T>(v: &Vec<T>) -> String
where
    T: ToString,
{
    let str_vec = v.iter().map(|fp| fp.to_string()).collect::<Vec<String>>();
    format!("[{}]", str_vec.join(", "))
}
