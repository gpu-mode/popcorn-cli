# Princeton 2026 Quick Start

Use `A100` and this leaderboard: `princeton_cross_entropy`.

```bash
# 1. Install the CLI
curl -fsSL https://raw.githubusercontent.com/gpu-mode/popcorn-cli/main/install.sh | bash

# 2. Register once with GitHub
popcorn register github

# 3. Join the closed leaderboard with your invite code
popcorn join <YOUR_INVITE_CODE>

# 4. Get the starter file
wget https://raw.githubusercontent.com/gpu-mode/reference-kernels/main/problems/princeton/cross_entropy_py/submission.py

# 5. Run a correctness check
popcorn submit --leaderboard princeton_cross_entropy --gpu A100 --mode test submission.py

# 6. Submit an official ranked run
popcorn submit --leaderboard princeton_cross_entropy --gpu A100 --mode leaderboard submission.py
```

Notes:

- `test` checks correctness only.
- `leaderboard` is the official ranked submission.
- If registration gets stuck, run `popcorn reregister github`.
