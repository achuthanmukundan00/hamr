import { homedir } from "node:os";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { validateProxyUrl } from "../src/core/http-dispatcher.ts";
import { PathGuard } from "../src/core/tools/path-guard.ts";

const HOME = homedir();

describe("PathGuard write confinement", () => {
	const guard = new PathGuard();

	it("allows writes inside the project cwd", () => {
		expect(guard.deniedWriteReason(join(process.cwd(), "src", "x.ts"))).toBeUndefined();
	});

	it("allows writes to the OS temp dir (macOS /var/folders, Linux /tmp)", () => {
		expect(guard.deniedWriteReason(join(require("node:os").tmpdir(), "hamr-bash-abc.log"))).toBeUndefined();
	});

	it("blocks writes to ~/.ssh (authorized_keys persistence)", () => {
		expect(guard.deniedWriteReason(join(HOME, ".ssh", "authorized_keys"))).toMatch(/\.ssh/);
	});

	it("blocks writes to ~/.bashrc / ~/.zshrc (shell rc persistence)", () => {
		expect(guard.deniedWriteReason(join(HOME, ".bashrc"))).toMatch(/sensitive file/);
		expect(guard.deniedWriteReason(join(HOME, ".zshrc"))).toMatch(/sensitive file/);
	});

	it("blocks writes to ~/.aws credentials", () => {
		expect(guard.deniedWriteReason(join(HOME, ".aws", "credentials"))).toMatch(/\.aws/);
	});

	it("blocks writes inside .git/hooks (git-hook persistence)", () => {
		expect(guard.deniedWriteReason(join(process.cwd(), ".git", "hooks", "post-commit"))).toMatch(/git hooks/);
	});

	it("blocks writes to /etc (system config)", () => {
		expect(guard.deniedWriteReason("/etc/cron.d/evil")).toMatch(/\/etc/);
	});

	it("can be disabled", () => {
		const off = new PathGuard({ enabled: false });
		expect(off.deniedWriteReason(join(HOME, ".ssh", "authorized_keys"))).toBeUndefined();
	});

	it("honors explicit allowedPaths override", () => {
		const g = new PathGuard({ allowedPaths: [join(HOME, ".ssh")] });
		expect(g.deniedWriteReason(join(HOME, ".ssh", "authorized_keys"))).toBeUndefined();
	});

	it("assertWritable throws PathGuardError", () => {
		expect(() => guard.assertWritable(join(HOME, ".ssh", "authorized_keys"))).toThrow();
	});
});

describe("PathGuard read confinement", () => {
	const guard = new PathGuard();

	it("blocks reads of SSH private keys", () => {
		expect(guard.deniedReadReason(join(HOME, ".ssh", "id_rsa"))).toMatch(/Refusing to read/);
		expect(guard.deniedReadReason(join(HOME, ".ssh", "id_ed25519"))).toMatch(/Refusing to read/);
	});

	it("blocks reads of ~/.aws/credentials", () => {
		expect(guard.deniedReadReason(join(HOME, ".aws", "credentials"))).toMatch(/credential file/);
	});

	it("blocks reads of hamr auth.json", () => {
		expect(guard.deniedReadReason(join(HOME, ".config", "hamr", "auth.json"))).toMatch(/credential file/);
	});

	it("allows reads of normal project files", () => {
		expect(guard.deniedReadReason(join(process.cwd(), "README.md"))).toBeUndefined();
	});

	it("allows reads of ~/.ssh/config and known_hosts (not private keys)", () => {
		expect(guard.deniedReadReason(join(HOME, ".ssh", "config"))).toBeUndefined();
		expect(guard.deniedReadReason(join(HOME, ".ssh", "known_hosts"))).toBeUndefined();
	});
});

describe("validateProxyUrl", () => {
	it("accepts http and https proxy URLs", () => {
		expect(validateProxyUrl("http://127.0.0.1:7890")).toBe("http://127.0.0.1:7890");
		expect(validateProxyUrl("https://proxy.example.com:8443")).toBe("https://proxy.example.com:8443");
	});

	it("returns undefined for empty/blank input", () => {
		expect(validateProxyUrl(undefined)).toBeUndefined();
		expect(validateProxyUrl("   ")).toBeUndefined();
	});

	it("rejects non-http schemes", () => {
		expect(() => validateProxyUrl("socks5://127.0.0.1:1080")).toThrow(/scheme/);
		expect(() => validateProxyUrl("file:///etc/passwd")).toThrow(/scheme/);
	});

	it("rejects malformed URLs", () => {
		expect(() => validateProxyUrl("not a url")).toThrow(/not a valid URL/);
	});

	it("rejects URLs without a host", () => {
		expect(() => validateProxyUrl("http://")).toThrow(/Invalid httpProxy/);
	});
});
