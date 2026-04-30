path = r'd:\vibe coding\Apartment\A1\apartment-manager\src-tauri\src\db\queries.rs'
with open(path, 'r', encoding='utf-8') as f:
    content = f.read()

L = '\n'

# Find pub fn query_bills_tx(
fn_marker = 'pub fn query_bills_tx('
fn_idx = content.find(fn_marker)
print(f'pub fn query_bills_tx at: {fn_idx}')

# Find the doc comment start (go back to find /// line)
# The doc comment starts with /// above the function
# Search backwards for "/// 事务版本"
doc_start = content.rfind('/// 事务版本', 0, fn_idx)
print(f'Doc comment start: {doc_start}')

# The doc comment spans until the line before pub fn
# Find the line before pub fn that's empty or is the doc comment end
end_of_doc = fn_idx
while end_of_doc > 0 and content[end_of_doc-1] == ' ':
    end_of_doc -= 1  # strip trailing spaces
# Find the last \n before pub fn
last_nl = content.rfind(L, 0, fn_idx)
print(f'Last newline before fn: {last_nl}')
# The doc comment ends at this newline (but includes the /// comment lines above)

# Actually the doc comment includes all lines starting with /// before pub fn
# Find the start of the first /// comment line
search_start = max(0, doc_start - 200)
segment = content[doc_start-20:fn_idx]
print(f'Segment: {repr(segment[-100:])}')

# The old function starts at doc_start and ends at "/// 账单行"
old_func_end = content.find('/// 账单行（操作前校验用）', doc_start)
print(f'Old function end: {old_func_end}')

old_func = content[doc_start:old_func_end]
print(f'Old function: {len(old_func)} chars')

# New function
new_func = (
    '/// 事务版本：分页查询账单列表（支持筛选）' + L +
    '///' + L +
    '/// 重构：消除 match 分支，使用动态 WHERE + params 向量' + L +
    'pub fn query_bills_tx(' + L +
    '    tx: &Transaction,' + L +
    '    year: Option<i32>,' + L +
    '    month: Option<i32>,' + L +
    '    room_id: Option<i64>,' + L +
    '    status: Option<&str>,' + L +
    '    page: i32,' + L +
    '    page_size: i32,' + L +
    ') -> Result<(Vec<BillRow>, i32)> {' + L +
    '    let mut where_clauses = vec!["mb.is_deleted = 0".to_string()];' + L +
    '    let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();' + L +
    L +
    '    if let Some(y) = year {' + L +
    '        if let Some(m) = month {' + L +
    '            where_clauses.push("mb.year_month = ?".to_string());' + L +
    '            params.push(Box::new(format!("{:04}-{:02}", y, m));' + L +
    '        } else {' + L +
    '            where_clauses.push("mb.year_month LIKE ?".to_string());' + L +
    '            params.push(Box::new(format!("{:04}-%", y));' + L +
    '        }' + L +
    '    }' + L +
    '    if let Some(r) = room_id {' + L +
    '        where_clauses.push("mb.room_id = ?".to_string());' + L +
    '        params.push(Box::new(r));' + L +
    '    }' + L +
    '    if let Some(s) = status {' + L +
    '        where_clauses.push("mb.status = ?".to_string());' + L +
    '        params.push(Box::new(s.to_string()));' + L +
    '    }' + L +
    L +
    '    let where_sql = where_clauses.join(" AND ");' + L +
    '    let offset = (page - 1) * page_size;' + L +
    L +
    '    let sql_count = format!("SELECT COUNT(*) FROM monthly_bills mb WHERE {}", where_sql);' + L +
    '    let mut stmt_count = tx.prepare(&sql_count)' + L +
    '        .map_err(crate::errors::AppError::Database)?;' + L +
    '    let count: i32 = stmt_count' + L +
    '        .query_row(rusqlite::params_from_iter(&params), |row| row.get(0))' + L +
    '        .map_err(crate::errors::AppError::Database)?;' + L +
    L +
    '    params.push(Box::new(page_size));' + L +
    '    params.push(Box::new(offset));' + L +
    L +
    '    let sql = format!(' + L +
    '        "SELECT mb.id, mb.room_id, mb.year_month, \\' + L +
    '                mb.total_amount, mb.actual_paid, mb.status, mb.due_date, mb.created_at, \\' + L +
    '                r.room_number, r.building, \\' + L +
    '                t.name as tenant_name \\' + L +
    '         FROM monthly_bills mb \\' + L +
    '         LEFT JOIN rooms r ON mb.room_id = r.id \\' + L +
    '         LEFT JOIN leases l ON mb.lease_id = l.id AND l.is_deleted = 0 \\' + L +
    '         LEFT JOIN tenants t ON l.tenant_id = t.id AND t.is_deleted = 0 \\' + L +
    '         WHERE {} \\' + L +
    '         ORDER BY mb.year_month DESC, mb.room_id \\' + L +
    '         LIMIT ? OFFSET ?",' + L +
    '        where_sql' + L +
    '    );' + L +
    '    let mut stmt = tx.prepare(&sql)' + L +
    '        .map_err(crate::errors::AppError::Database)?;' + L +
    '    let bills: Vec<BillRow> = stmt' + L +
    '        .query_map(rusqlite::params_from_iter(&params), BillRow::from_row)' + L +
    '        .map_err(crate::errors::AppError::Database)?' + L +
    '        .collect::<SqliteResult<Vec<_>>>()' + L +
    '        .map_err(crate::errors::AppError::Database)?;' + L +
    L +
    '    Ok((bills, count))' + L +
    '}' + L
)

new_content = content[:doc_start] + new_func + L + L + content[old_func_end:]
print(f'New content: {len(new_content)} chars')

with open(path, 'w', encoding='utf-8') as f:
    f.write(new_content)
print('Written')
