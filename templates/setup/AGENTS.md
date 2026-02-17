## Skills
A skill is a local instruction bundle stored in `SKILL.md`.

### Available skills
- {{SKILL_NAME}}: Helps with Popcorn CLI registration, submission setup, submission modes, and file directives. (file: {{SKILL_PATH}})
- {{NATIVE_SKILL_NAME}}: Helps write CUDA and HIP kernels using torch.utils.cpp_extension.load_inline(). Use when writing native GPU code inside a Python submission. (file: {{NATIVE_SKILL_PATH}})

### How to use skills
- Load the skill by reading its `SKILL.md` file when user requests match the description.
- Follow progressive disclosure: read only relevant referenced files/scripts as needed.
- Keep the workspace setup aligned with `popcorn setup`.
