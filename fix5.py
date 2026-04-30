path = r'd:\vibe coding\Apartment\A1\apartment-manager\src-tauri\src\services\bill_service.rs'
content = open(path, 'rb').read().decode('utf-8', errors='replace')
content = content.replace('\r\n', '\n').replace('\r', '\n')

# Fix 1: partial_pay_bill_tx call - add new_actual_paid and new_status variables
old1 = '            queries::partial_pay_bill_tx(tx, bill_id, amount, new_status.as_str())?'
new1 = ('            let new_actual_paid = bill.actual_paid.checked_add(amount)\n'
            '                .ok_or_else(|| AppError::business("部分支付后已付金额计算溢出"))?;\n'
            '            let new_status = BillStatus::from_paid_amount(bill.total_amount, new_actual_paid);\n'
            '            queries::partial_pay_bill_tx(tx, bill_id, new_actual_paid, new_status.as_str())?')
if old1 in content:
    content = content.replace(old1, new1, 1)
    print('Fixed partial_pay_bill_tx')
else:
    print('WARNING: partial_pay_bill_tx pattern not found')

# Fix 2: Add get_bill_summary method to BillService
old2 = '            queries::list_archived_year_months(&conn)\n        })\n    }\n\n#[cfg(test)]'
new2 = ('            queries::list_archived_year_months(&conn)\n'
    '        })\n'
    '\n'
    '    }\n'
    '\n'
    '    pub fn get_bill_summary<C: HasConnection>(\n'
    '        &self,\n'
    '        ctx: &C,\n'
    '        year_month: Option<&str>,\n'
    '    ) -> Result<BillSummary> {\n'
    '        let conn = ctx.get_conn()?;\n'
    '        queries::get_bill_summary(&conn, year_month)\n'
    '    }\n'
    '\n'
    '#[cfg(test)]')
if old2 in content:
    content = content.replace(old2, new2, 1)
    print('Added get_bill_summary method')
else:
    print('WARNING: get_bill_summary insertion point not found')
    idx = content.find('list_archived_year_months')
    print(f'list_archived_year_months at idx {idx}')
    snippet = content[idx:idx+100]
    print('Snippet:', repr(snippet[:80]))

with open(path, 'w', encoding='utf-8') as f:
    f.write(content)
print('Written')
