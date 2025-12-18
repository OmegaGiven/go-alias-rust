
use crate::sql::DbConnection;

pub fn find_connection<'a>(nick: &str, conns: &'a [DbConnection]) -> Option<&'a DbConnection> {
    conns.iter().find(|c| c.nickname == nick)
}


pub fn html_escape(s: &str) -> String {
    htmlescape::encode_minimal(s)
}

// MODIFIED: Function signature updated to accept ordered headers and Vec<Vec<String>> data
pub fn render_table(headers: &[String], results: &[Vec<String>]) -> String {
    if results.is_empty() || headers.is_empty() {
        return "<pre>No output yet.</pre>".to_string();
    }
    
    // Use the provided headers (already sorted by DB order)
    let thead = format!(
        "<tr>{}</tr>",
        headers.iter()
            .map(|h| format!("<th>{}</th>", html_escape(h)))
            .collect::<Vec<_>>()
            .join("")
    );
    
    // Iterate over the ordered rows
    let tbody = results.iter().map(|row| {
        let tds = row.iter()
            .map(|v| format!("<td>{}</td>", html_escape(v)))
            .collect::<Vec<_>>()
            .join("");
        format!("<tr>{}</tr>", tds)
    }).collect::<Vec<_>>().join("");

    format!(
        "<table class=\"grid\" style=\"margin: 0;\"><thead>{}</thead><tbody>{}</tbody></table>",
        thead, tbody
    )
}