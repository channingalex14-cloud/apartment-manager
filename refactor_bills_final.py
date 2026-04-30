# Clean refactoring of query_bills_tx using Vec<Box<dyn ToSql>>

path = r'd:\vibe coding\Apartment\A1\apartment-manager\src-tauri\src\db\queries.rs'
with open(path, 'r', encoding='utf-8') as f:
    content = f.read()

# Find the exact start and end of the old function
fn_start_marker = 'pub fn query_bills_tx('
fn_start = content.find(fn_start_marker)
doc_end_marker = '\n\npub fn query_bills_tx'
# Go back from fn_start to find the doc comment
search_from = content.rfind('\n/// ', 0, fn_start)
doc_start = content.find('/// 事务版本', search_from)
if doc_start < 0:
    doc_start = search_from

doc_end = content.find('pub fn query_bills_tx', doc_start)
print(f'Doc start: {doc_start}, Doc end: {doc_end}')

# Find function end
# The function ends before "/// 账单行（操作前校验用）"
func_body_start = fn_start  # doc comment ends just before this
func_end = content.find('/// 账单行（操作前校验用）', fn_start)
print(f'Func start: {func_body_start}, Func end: {func_end}')

old_text = content[doc_start:func_end]
print(f'Old text: {len(old_text)} chars')

# Build the new function text using a template approach
# Since Python string formatting with {} is problematic, use simple concatenation
NL = '\n'

new_func = (
    '/// 事务版本：分页查询账单列表（支持筛选）' + NL +
    '///' + NL +
    '/// 重构：消除 match 分支，使用动态 WHERE + params 向量' + NL +
    'pub fn query_bills_tx(' + NL +
    '    tx: &Transaction,' + NL +
    '    year: Option<i32>,' + NL +
    '    month: Option<i32>,' + NL +
    '    room_id: Option<i64>,' + NL +
    '    status: Option<&str>,' + NL +
    '    page: i32,' + NL +
    '    page_size: i32,' + NL +
    ') -> Result<(Vec<BillRow>, i32)> {' + NL +
    '    let mut where_clauses = vec!["mb.is_deleted = 0".to_string()];' + NL +
    '    let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();' + NL +
    NL +
    '    if let Some(y) = year {' + NL +
    '        if let Some(m) = month {' + NL +
    '            where_clauses.push("mb.year_month = ?".to_string());' + NL +
    '            params.push(Box::new(format!("{:04}-{:02}", y, m)));' + NL +
    '        } else {' + NL +
    '            where_clauses.push("mb.year_month LIKE ?".to_string());' + NL +
    '            params.push(Box::new(format!("{:04}-%", y)));' + NL +
    '        }' + NL +
    '    }' + NL +
    '    if let Some(r) = room_id {' + NL +
    '        where_clauses.push("mb.room_id = ?".to_string());' + NL +
    '        params.push(Box::new(r));' + NL +
    '    }' + NL +
    '    if let Some(s) = status {' + NL +
    '        where_clauses.push("mb.status = ?".to_string());' + NL +
    '        params.push(Box::new(s.to_string()));' + NL +
    '    }' + NL +
    NL +
    '    let where_sql = where_clauses.join(" AND ");' + NL +
    '    let offset = (page - 1) * page_size;' + NL +
    NL +
    '    let sql_count = format!("SELECT COUNT(*) FROM monthly_bills mb WHERE {}", where_sql);' + NL +
    '    let count: i32 = {' + NL +
    '        let mut stmt = tx.prepare(&sql_count)' + NL +
    '            .map_err(crate::errors::AppError::Database)?;' + NL +
    '        stmt.query_row(params_from_iter(params.iter()), |row| row.get(0))' + NL +
    '            .map_err(crate::errors::AppError::Database)?' + NL +
    '    };' + NL +
    NL +
    '    let mut data_params = params;' + NL +
    '    data_params.push(Box::new(page_size));' + NL +
    '    data_params.push(Box::new(offset));' + NL +
    NL +
    '    let sql = format!(' + NL +
    '        "SELECT mb.id, mb.room_id, mb.year_month, \\"' + NL +
    '               mb.total_amount, mb.actual_paid, mb.status, mb.due_date, mb.created_at, \\"' + NL +
    '               r.room_number, r.building, \\"' + NL +
    '               t.name as tenant_name \\"' + NL +
    '         FROM monthly_bills mb \\"' + NL +
    '         LEFT JOIN rooms r ON mb.room_id = r.id \\"' + NL +
    '         LEFT JOIN leases l ON mb.lease_id = l.id AND l.is_deleted = 0 \\"' + NL +
    '         LEFT JOIN tenants t ON l.tenant_id = t.id AND t.is_deleted = 0 \\"' + NL +
    '         WHERE {} \\"' + NL +
    '         ORDER BY mb.year_month DESC, mb.room_id \\"' + NL +
    '         LIMIT ? OFFSET ?",' + NL +
    '        where_sql' + NL +
    '    );' + NL +
    '    let bills: Vec<BillRow> = {' + NL +
    '        let mut stmt = tx.prepare(&sql)' + NL +
    '            .map_err(crate::errors::AppError::Database)?;' + NL +
    '        stmt.query_map(params_from_iter(data_params.iter()), BillRow::from_row)' + NL +
    '            .map_err(crate::errors::AppError::Database)?' + NL +
    '            .collect::<SqliteResult<Vec<_>>>()' + NL +
    '            .map_err(crate::errors::AppError::Database)?' + NL +
    '    };' + NL +
    NL +
    '    Ok((bills, count))' + NL +
    '}' + NL
)

new_content = content[:doc_start] + new_func + NL + NL + content[func_end:]

with open(path, 'w', encoding='utf-8') as f:
    f.write(new_content)
print(f'Written: old={len(old_text)}, new={len(new_func)}, diff={len(new_func)-len(old_text)}')
