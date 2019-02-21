import * as native from "./native";
import * as vscode from 'vscode';
import * as path from 'path';

export class Panel {
	private panel: vscode.WebviewPanel | null;
	private extensionPath: string;
	public constructor(extensionPath: string) {
		this.panel = null;
		this.extensionPath = extensionPath;
	}
	public focus(): void {
		this.get().reveal();
	}
	public update(tree: native.TestviewTree): void {
		this.get().webview.html = this.view(tree);
	}
	private get(): vscode.WebviewPanel {
		return this.panel || this.create();
	}
	private create(): vscode.WebviewPanel {
		this.panel = vscode.window.createWebviewPanel(
			'icie test view',
			'ICIE Test View',
			vscode.ViewColumn.One,
			{
				enableScripts: false
			}
		);
		this.panel.onDidDispose(() => this.panel = null);
		return this.panel;
	}
	private view(tree: native.TestviewTree): string {
		return `
			<html>
				<head>
					<link rel="stylesheet" href="${this.asset('web', 'testview.css')}">
					<link href="https://fonts.googleapis.com/icon?family=Material+Icons" rel="stylesheet">
				</head>
				<body>
					<table class="test">
						${this.viewTree(tree)}
					</table>
				</body>
			</html>
		`;
	}
	private viewTree(tree: native.TestviewTree): string {
		if (native.isTest(tree)) {
			let rows = Math.max(...[tree.input, tree.output].map(lines));
			if (tree.desired !== null) {
				rows = Math.max(rows, lines(tree.desired));
			}
			let good = tree.output.trim() === (tree.desired || "").trim();
			return `
				</tr>
					<td class="data">
						<div class="info">
							<i class="material-icons" title=${tree.name}>info</i>
						</div>
						${tree.input.replace('\n', '<br/>')}
					</td>
					<td class="data ${good ? "out-good" : "out-bad"}">${tree.output.replace('\n', '<br/>')}</td>
					<td class="data">${(tree.desired || "").replace('\n', '<br/>')}</td>
				</tr>
			`;
		} else {
			return `
				${tree.map(tree2 => this.viewTree(tree2)).join('\n')}
			`;
		}
	}
	private asset(...parts: string[]): vscode.Uri {
		return vscode.Uri.file(path.join(this.extensionPath, 'assets', ...parts)).with({ scheme: 'vscode-resource' });
	}
}

function lines(text: string): number {
	return text.split('\n').length;
}