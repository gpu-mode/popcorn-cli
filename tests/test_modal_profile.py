"""CPU checks for the embedded runner; importing the full module builds a Modal image."""
import ast
import unittest
from pathlib import Path

RUNNER = Path(__file__).resolve().parents[1] / "templates/local_modal_runner.py"
namespace = {"Path": Path}
tree = ast.parse(RUNNER.read_text())
helpers = ast.Module(body=[node for node in tree.body if isinstance(node, ast.FunctionDef)
                          and node.name in {"_ncu_command", "_select_benchmarks"}], type_ignores=[])
exec(compile(helpers, str(RUNNER), "exec"), namespace)


class ModalProfileTests(unittest.TestCase):
    def test_selected_shape_and_all_shapes(self):
        config = {"benchmarks": [{"n": 32}, {"n": 512}], "tests": [{"n": 16}]}
        select = namespace["_select_benchmarks"]
        self.assertIs(select(config, {}), config)
        self.assertEqual(select(config, {"benchmark_index": 1})["benchmarks"], [{"n": 512}])
        self.assertEqual(len(config["benchmarks"]), 2)
        self.assertEqual(select(config, {"benchmark_index": 1})["tests"], [{"n": 16}])
        for index in [-1, 2]:
            with self.assertRaises(ValueError):
                select(config, {"benchmark_index": index})
        with self.assertRaises(ValueError):
            select({"benchmarks": []}, {})

    def test_capture_tracks_child_processes_without_clock_control(self):
        command = namespace["_ncu_command"](["python3", "eval.py"], Path("/tmp/out"), {})
        self.assertEqual(command[-3:], ["--", "python3", "eval.py"])
        for flag, value in [("--clock-control", "none"), ("--target-processes", "all"),
                            ("--nvtx-include", "custom_kernel/"), ("--launch-count", "10")]:
            self.assertEqual(command[command.index(flag) + 1], value)

    def test_filters_remain_single_arguments(self):
        command = namespace["_ncu_command"](["program"], Path("/tmp/out"), {
            "ncu_kernel_name": "regex:late_kernel|kernel with spaces",
            "ncu_kernel_name_base": "demangled", "ncu_launch_count": 2,
        })
        self.assertEqual(command[command.index("--kernel-name") + 1], "regex:late_kernel|kernel with spaces")
        self.assertEqual(command[command.index("--kernel-name-base") + 1], "demangled")
        self.assertEqual(command[command.index("--launch-count") + 1], "2")


if __name__ == "__main__":
    unittest.main()
