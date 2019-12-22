package nnn

import (
	"tool/file"
)

command adopt: {
	// repository containing copies of files expected to exist in the filesystem
	//var: shadow: "."
	var: shadow: "."

	// TODO: usage, short, long
	//task: render: {
	//task render: file.Create & {
	//	filename: "\(shadow)/
	//}
	task: {
		for filename, contents in shadowMap 
		if contents.kind == "present" {
			"render \(filename)": file.Create & {
				filename: "\(var.shadow)/\(filename)",
				contents: contents.text
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
