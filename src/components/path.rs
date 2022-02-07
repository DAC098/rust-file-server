pub fn join_id_and_path(users_id: &i64, context: &str) -> String {
    let id_str = users_id.to_string();

    if context.len() == 0 {
        id_str
    } else {
        let mut rtn = String::with_capacity(id_str.len() + 1 + context.len());
        rtn.push_str(&id_str);
        rtn.push('/');
        rtn.push_str(context);
        rtn
    }
}