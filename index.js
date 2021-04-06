const hl = import('./pkg');

const file = "package main\n\nimport (\n    \"fmt\"\n    \"time\"\n)\n\nfunc f(from string) {\n    for i := 0; i < 3; i++ {\n        fmt.Println(from, \":\", i)\n    }\n}\n\nfunc main() {\n\n    f(\"direct\")\n\n    go f(\"goroutine\")\n\n    go func(msg string) {\n        fmt.Println(msg)\n    }(\"going\")\n\n    time.Sleep(time.Second)\n    fmt.Println(\"done\")\n}";

hl.then(m => {
		const result = m.highlight_file(file, "test.go", true, true);
		console.log(result)
}).catch(console.error);
