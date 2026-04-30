path = r'd:\vibe coding\Apartment\A1\apartment-manager\src-tauri\src\db\queries.rs'
with open(path, 'rb') as f:
    raw = f.read()
content = raw.decode('utf-8', errors='replace')

nl = '\n'

new_body = f'''{nl}/// 事务版本：分页查询账单列表（支持筛选）
///
/// 重构：消除 match 分支，使用动态 WHERE + params 向量
pub fn query_bills_tx(
    tx: &Transaction,
    year: Option<i32>,
    month: Option<i32>,
    room_id: Option<i64>,
    status: Option<&str>,
    page: i32,
    page_size: i32,
) -> Result<(Vec<BillRow>, i32)> {{
    let mut where_clauses = vec!["mb.is_deleted = 0".to_string()];
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(y) = year {{
        if let Some(m) = month {{
            where_clauses.push("mb.year_month = ?".to_string());
            params.push(Box::new(format!("{{:04}}-{{:02}}", y, m)));
        }} else {{
            where_clauses.push("mb.year_month LIKE ?".to_string());
            params.push(Box::new(format!("{{:04}}-%", y)));
        }}
    }}
    if let Some(r) = room_id {{
        where_clauses.push("mb.room_id = ?".to_string());
        params.push(Box::new(r));
    }}
    if let Some(s) = status {{
        where_clauses.push("mb.status = ?".to_string());
        params.push(Box::new(s.to_string()));
    }}

    let where_sql = where_clauses.join(" AND ");
    let offset = (page - 1) * page_size;

    let sql_count = format!("SELECT COUNT(*) FROM monthly_bills mb WHERE {{}}", where_sql);
    let mut stmt_count = tx.prepare(&sql_count)
        .map_err(crate::errors::AppError::Database)?;
    let count: i32 = stmt_count
        .query_row(rusqlite::params_from_iter(&params), |row| row.get(0))
        .map_err(crate::errors::AppError::Database)?;

    params.push(Box::new(page_size));
    params.push(Box::new(offset));

    let sql = format!(
        "SELECT mb.id, mb.room_id, mb.year_month, \\
                mb.total_amount, mb.actual_paid, mb.status, mb.due_date, mb.created_at, \\
                r.room_number, r.building, \\
                t.name as tenant_name \\
         FROM monthly_bills mb \\
         LEFT JOIN rooms r ON mb.room_id = r.id \\
         LEFT JOIN leases l ON mb.lease_id = l.id AND l.is_deleted = 0 \\
         LEFT JOIN tenants t ON l.tenant_id = t.id AND t.is_deleted = 0 \\
         WHERE {{}} \\
         ORDER BY mb.year_month DESC, mb.room_id \\
         LIMIT ? OFFSET ?",
        where_sql
    );
    let mut stmt = tx.prepare(&sql)
        .map_err(crate::errors::AppError::Database)?;
    let bills: Vec<BillRow> = stmt
        .query_map(rusqlite::params_from_iter(&params), BillRow::from_row)
        .map_err(crate::errors::AppError::Database)?
        .collect::<SqliteResult<Vec<_>>>()
        .map_err(crate::errors::AppError::Database)?;

    Ok((bills, count))
}}
{nl}
{nl}
'''

# Find the function boundaries
old_marker = '/// 事务版本：分页查询账单列表（支持筛选）'
end_marker = '/// 账单行（操作前校验用）'

idx_start = content.find(old_marker)
idx_end = content.find(end_marker, idx_start)
print(f'Start: {idx_start}, End: {idx_end}')
print(f'Content between: {repr(content[idx_start:idx_start+100])}')

old_text = content[idx_start:idx_end]
print(f'Old text length: {len(old_text)}')

new_text = content[:idx_start] + new_body.strip() + '\n\n' + content[idx_end:]
print(f'New content length: {len(new_text)}')

with open(path, 'w', encoding='utf-8') as f:
    f.write(new_text)
print('Written')
