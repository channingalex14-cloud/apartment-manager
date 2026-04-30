path = r'd:\vibe coding\Apartment\A1\apartment-manager\src-tauri\src\db\queries.rs'
with open(path, 'r', encoding='utf-8') as f:
    content = f.read()

# Find the doc comment start and function body start/end
doc_comment = '/// 事务版本：分页查询账单列表（支持筛选）'
fn_def = 'pub fn query_bills_tx('
bill_row_comment = '/// 账单行（操作前校验用）'

doc_start = content.find(doc_comment)
fn_start = content.find(fn_def, doc_start)
func_end = content.find(bill_row_comment, fn_start)

print(f'doc_start: {doc_start}, fn_start: {fn_start}, func_end: {func_end}')

# The old text is from doc_start to func_end
old_text = content[doc_start:func_end]
print(f'Old text: {len(old_text)} chars, {old_text.count(chr(10))} lines')

# Check what the new function should look like
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

print(f'New function: {len(new_func)} chars')

new_content = content[:doc_start] + new_func + NL + NL + content[func_end:]

with open(path, 'w', encoding='utf-8') as f:
    f.write(new_content)
print(f'Done. New content: {len(new_content)} chars')
