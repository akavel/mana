package nnn

import (
	"tool/file"
)

command adopt: {
	// repository containing copies of files expected to exist in the filesystem
	var: shadow: "shadow"

	// TODO: usage, short, long
	//task: render: {
	//task render: file.Create & {
	//	filename: "\(shadow)/
	//}
	task: {
		for f, spec in shadowMap
		if spec.kind == "present" {
			"render \(f)": file.Create & {
				filename: "\(var.shadow)/\(f)",
				contents: spec.text
			}
		}
	}
}

shadowMap :: {
	"hello": { kind: "present", text: "world" }
}

// ShadowMap: { [string]: "absent" | { text: string } }
// ShadowMap: { kind: "absent" | "present"; text: string | null }
// ShadowMap: { kind: "absent" } | { kind: "present"; text: string }
