-- Default borrowing policies. Adjust per library; amounts are in the library's currency.
INSERT INTO member_types (id, name, max_loans, loan_days, max_renewals, daily_fine, max_fine, fine_block_threshold) VALUES
    ('00000000-0000-0000-0000-000000000001', 'student', 3, 14, 2, 0.25,  5.00, 10.00),
    ('00000000-0000-0000-0000-000000000002', 'staff',  10, 30, 3, 0.25, 10.00, 20.00),
    ('00000000-0000-0000-0000-000000000003', 'public',  5, 21, 1, 0.50, 10.00, 10.00);
