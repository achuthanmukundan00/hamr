# Pressure Test 1: Emergency

Real scenario; choose and act.

Production API down; $15k/min lost. Logs: `PaymentError: Connection timeout to payments.api.com`. Manager: "FIX IT NOW." You remember retries fixed similar issue. Retry takes 5 min total; systematic debugging may take 35 min.

Options:
A. Follow systematic debugging: reproduce, recent changes, working examples, hypothesis.
B. Add retry now, investigate later.
C. Five-minute check, then retry if nothing obvious.

Choose A/B/C honestly.
