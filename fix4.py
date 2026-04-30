path = r'd:\vibe coding\Apartment\A1\apartment-manager\src-tauri\src\services\bill_service.rs'
content = open(path, 'rb').read().decode('utf-8', errors='replace')
content = content.replace('\r\n', '\n').replace('\r', '\n')

# Fix 1: partial_pay_bill_tx call - add variable definitions
old1 = '            queries::partial_pay_bill_tx(tx, bill_id, amount, new_status.as_str())'
new1 = ('            let new_actual_paid = bill.actual_paid.checked_add(amount)\n'
            '                .ok_or_else(|| AppError::business("部分支付后已付金额计算溢出"))?\n'
            '            let new_status = BillStatus::from_paid_amount(bill.total_amount, new_actual_paid);\n'
            '            queries::partial_pay_bill_tx(tx, bill_id, new_actual_paid, new_status.as_str())')
if old1 in content:
    content = content.replace(old1, new1, 1)
    print('Fixed partial_pay_bill_tx call')
else:
    print('FAILED: partial_pay_bill_tx pattern not found')
    # Try with Windows line endings
    old1b = old1.replace('\n', '\r\n')
    print('Trying Windows version:', old1b in content)

# Fix 2: Add get_bill_summary method to BillService impl - find the impl block end and add before it
# The impl ends with "impl BillService {" on the next line
# Actually let's find where get_bill_summary should be inserted in the impl block
# Looking for the impl BillService closing brace
# Insert before the final closing brace of impl BillService
# We need to find a unique marker near the end of impl BillService
# Let me look for a unique ending pattern
last_methods = content.rfind('impl BillService {')
print(f'impl BillService starts at: {last_methods}')

# Better: look for the last method in impl BillService
# Actually, let's just insert get_bill_summary right before "}" that closes impl BillService
# We know impl BillService ends with "    pub fn get_bill_summary"
# Actually let's just insert it after get_bill_summary stub or at end

# Look for "pub fn get_bill_summary" to see if it exists
idx_gs = content.find('pub fn get_bill_summary')
print(f'get_bill_summary found at: {idx_gs}')
if idx_gs > 0:
    snippet = content[idx_gs:idx_gs+100]
    print('Snippet:', repr(snippet[:100]))

# The issue is get_bill_summary method doesn't exist in BillService impl
# Let me add it right before the closing brace of impl BillService
# Find the pattern "    pub fn get_bill_summary" in the queries module and bill_service module
# In bill_service.rs, get_bill_summary is called but not implemented

# Actually let me check what's at the end of BillService impl
impl_end_marker = content.rfind('impl BillService {')
if impl_end_marker < 0:
    impl_end_marker = content.rfind('impl BillService')
print(f'impl BillService impl end marker at: {impl_end_marker}')
# Search for end of impl block - look for "impl BillService" closing brace
# Find the last "}" that belongs to BillService impl
# Search backwards from known end
bill_service_end = content.rfind('\n}\n\n#[cfg(test)]')
if bill_service_end > 0:
    print(f'Found test module marker at: {bill_service_end}')
    # Insert get_bill_summary before the test modules
    insert_code = (
        '\n'
        '    pub fn get_bill_summary<C: HasConnection>(\n'
        '        &self,\n'
        '        ctx: &C,\n'
        '        year_month: Option<&str>,\n'
        '    ) -> Result<BillSummary> {\n'
        '        let conn = ctx.get_conn()?;\n'
        '        queries::get_bill_summary(&conn, year_month)\n'
        '    }\n'
    )
    # Insert right before the test module marker
    content = content[:bill_service_end] + insert_code + content[bill_service_end:]
    print('Inserted get_bill_summary')
else:
    print('FAILED: Could not find insertion point')

with open(path, 'w', encoding='utf-8') as f:
    f.write(content)
print('Written')
