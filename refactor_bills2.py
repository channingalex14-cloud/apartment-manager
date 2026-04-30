# This script does a surgical replacement of query_bills_tx function

path = r'd:\vibe coding\Apartment\A1\apartment-manager\src-tauri\src\db\queries.rs'
with open(path, 'r', encoding='utf-8') as f:
    content = f.read()

# Find the old function
old_func_start = content.find('/// 事务版本：分页查询账单列表（支持筛选）\n\n\npub fn query_bills_tx(')
if old_func_start < 0:
    # Try double newline variant
    old_func_start = content.find('/// 事务版本：分页查询账单列表（支持筛选）\n\npub fn query_bills_tx(')
print(f'Function start: {old_func_start}')

old_func_end = content.find('/// 账单行（操作前校验用）', old_func_start)
print(f'Function end: {old_func_end}')

if old_func_start < 0 or old_func_end < 0:
    print('ERROR: could not find boundaries')
    exit(1)

old_func = content[old_func_start:old_func_end]
print(f'Old function: {len(old_func)} chars, {old_func.count(chr(10))} lines')

# New function body - use string concatenation to avoid f-string issues
L = '\n'
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
    '            params.push(Box::new(format!("{:04}-{:02}", y, m)));' + L +
    '        } else {' + L +
    '            where_clauses.push("mb.year_month LIKE ?".to_string());' + L +
    '            params.push(Box::new(format!("{:04}-%", y)));' + L +
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

print(f'New function: {len(new_func)} chars, {new_func.count(L)} lines')

# Replace
new_content = content[:old_func_start] + new_func + L + L + content[old_func_end:]
print(f'New content: {len(new_content)} chars')

with open(path, 'w', encoding='utf-8') as f:
    f.write(new_content)
print('Done')
