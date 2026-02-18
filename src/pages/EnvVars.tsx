import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Select } from "@/components/ui/select";
import { deleteEnvVars, getEnvVars, importEnvFile, listAllEnvVars, scanLocalSkills, setEnvVars } from "@/lib/api";
import { useConfigStore } from "@/lib/store";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import {
	Eye,
	EyeOff,
	FileInput,
	Key,
	Loader2,
	Package,
	Pencil,
	Plus,
	Save,
	Trash2,
} from "lucide-react";
import { useCallback, useEffect, useState } from "react";

interface EnvEntry {
	key: string;
	value: string;
	masked: boolean;
}

export function EnvVars() {
	const org = useConfigStore((s) => s.config.org);

	const [skills, setSkills] = useState<string[]>([]);
	const [selectedSkill, setSelectedSkill] = useState("");
	const [entries, setEntries] = useState<EnvEntry[]>([]);
	const [loading, setLoading] = useState(true);
	const [saving, setSaving] = useState(false);

	// New entry
	const [newKey, setNewKey] = useState("");
	const [newValue, setNewValue] = useState("");

	const loadSkills = useCallback(async () => {
		if (!org) return;
		setLoading(true);
		try {
			const [all, installed] = await Promise.all([
				listAllEnvVars(org),
				scanLocalSkills(),
			]);
			const installedNames = new Set(
				installed.map((s) => s.name.toLowerCase()),
			);
			const skillNames = all
				.map((s) => s.skill)
				.filter((name) => installedNames.has(name.toLowerCase()));
			setSkills(skillNames);
			if (skillNames.length > 0 && !selectedSkill) {
				setSelectedSkill(skillNames[0]);
			}
		} catch {
			// Non-blocking
		} finally {
			setLoading(false);
		}
	}, [org, selectedSkill]);

	const loadVars = useCallback(async () => {
		if (!org || !selectedSkill) {
			setEntries([]);
			return;
		}
		try {
			const vars = await getEnvVars(org, selectedSkill);
			setEntries(
				Object.entries(vars).map(([key, value]) => ({
					key,
					value,
					masked: true,
				})),
			);
		} catch {
			setEntries([]);
		}
	}, [org, selectedSkill]);

	useEffect(() => {
		loadSkills();
	}, [loadSkills]);

	useEffect(() => {
		loadVars();
	}, [loadVars]);

	const handleAdd = async () => {
		if (!org || !selectedSkill || !newKey.trim()) return;
		setSaving(true);
		try {
			await setEnvVars(org, selectedSkill, { [newKey.trim()]: newValue });
			setNewKey("");
			setNewValue("");
			await loadVars();
		} finally {
			setSaving(false);
		}
	};

	const handleDelete = async (key: string) => {
		if (!org || !selectedSkill) return;
		setSaving(true);
		try {
			await deleteEnvVars(org, selectedSkill, [key]);
			await loadVars();
		} finally {
			setSaving(false);
		}
	};

	const handleSave = async (key: string, value: string) => {
		if (!org || !selectedSkill) return;
		setSaving(true);
		try {
			await setEnvVars(org, selectedSkill, { [key]: value });
			await loadVars();
		} finally {
			setSaving(false);
		}
	};

	const handleImport = async () => {
		if (!org || !selectedSkill) return;
		try {
			const selected = await openDialog({
				filters: [{ name: "Env files", extensions: ["env"] }],
			});
			if (selected) {
				await importEnvFile(org, selectedSkill, selected as string);
				await loadVars();
			}
		} catch {
			// Dialog not available
		}
	};

	const toggleMask = (index: number) => {
		setEntries((prev) => prev.map((e, i) => (i === index ? { ...e, masked: !e.masked } : e)));
	};

	if (!org) {
		return (
			<div className="flex flex-col items-center justify-center gap-3 h-full">
				<Package className="size-10 text-muted-foreground" />
				<p className="text-muted-foreground">Select an organization first</p>
			</div>
		);
	}

	if (loading) {
		return (
			<div className="flex items-center justify-center h-full">
				<Loader2 className="size-6 animate-spin text-muted-foreground" />
			</div>
		);
	}

	return (
		<div className="flex flex-col gap-6 p-6 max-w-2xl">
			<h1 className="text-lg font-semibold">Environment Variables</h1>

			{/* Skill selector */}
			<div className="flex items-center gap-3">
				<div className="flex-1 space-y-1.5">
					<Label>Skill</Label>
					{skills.length > 0 ? (
						<Select value={selectedSkill} onChange={(e) => setSelectedSkill(e.target.value)}>
							{skills.map((s) => (
								<option key={s} value={s}>
									{s}
								</option>
							))}
						</Select>
					) : (
						<Input
							placeholder="Enter skill name"
							value={selectedSkill}
							onChange={(e) => setSelectedSkill(e.target.value)}
						/>
					)}
				</div>
				<div className="pt-6">
					<Button variant="outline" size="sm" onClick={handleImport}>
						<FileInput className="size-3.5" />
						Import .env
					</Button>
				</div>
			</div>

			{/* Vars table */}
			{selectedSkill && (
				<div className="rounded-xl border bg-card overflow-hidden">
					{/* Header */}
					<div className="grid grid-cols-[1fr_1fr_auto] gap-2 border-b px-4 py-2 text-xs font-medium text-muted-foreground">
						<span>Key</span>
						<span>Value</span>
						<span className="w-20" />
					</div>

					{/* Entries */}
					{entries.length > 0 ? (
						entries.map((entry, i) => (
							<EnvRow
								key={entry.key}
								entry={entry}
								onToggleMask={() => toggleMask(i)}
								onDelete={() => handleDelete(entry.key)}
								onSave={(value) => handleSave(entry.key, value)}
								disabled={saving}
							/>
						))
					) : (
						<div className="flex items-center justify-center gap-2 py-8 text-sm text-muted-foreground">
							<Key className="size-4" />
							No variables configured
						</div>
					)}

					{/* Add new */}
					<div className="grid grid-cols-[1fr_1fr_auto] gap-2 border-t px-4 py-3">
						<Input
							placeholder="KEY_NAME"
							value={newKey}
							onChange={(e) => setNewKey(e.target.value.toUpperCase())}
							className="font-mono text-xs"
						/>
						<Input
							placeholder="value"
							value={newValue}
							onChange={(e) => setNewValue(e.target.value)}
							className="text-xs"
						/>
						<Button size="sm" disabled={!newKey.trim() || saving} onClick={handleAdd}>
							<Plus className="size-3.5" />
							Add
						</Button>
					</div>
				</div>
			)}
		</div>
	);
}

function EnvRow({
	entry,
	onToggleMask,
	onDelete,
	onSave,
	disabled,
}: {
	entry: EnvEntry;
	onToggleMask: () => void;
	onDelete: () => void;
	onSave: (value: string) => void;
	disabled: boolean;
}) {
	const [editing, setEditing] = useState(false);
	const [value, setValue] = useState(entry.value);

	const maskedValue =
		entry.masked && entry.value.length > 4 ? `${entry.value.slice(0, 4)}****` : entry.value;

	return (
		<div className="grid grid-cols-[1fr_1fr_auto] gap-2 items-center border-b last:border-0 px-4 py-2">
			<span className="font-mono text-xs font-medium truncate">{entry.key}</span>
			{editing ? (
				<div className="flex gap-1">
					<Input
						value={value}
						onChange={(e) => setValue(e.target.value)}
						className="text-xs h-7"
						autoFocus
					/>
					<Button
						size="sm"
						variant="ghost"
						className="h-7 w-7 p-0"
						onClick={() => {
							onSave(value);
							setEditing(false);
						}}
					>
						<Save className="size-3" />
					</Button>
				</div>
			) : (
				<span className="font-mono text-xs text-muted-foreground truncate">{maskedValue}</span>
			)}
			<div className="flex items-center gap-1 w-20 justify-end">
				<button
					type="button"
					onClick={onToggleMask}
					className="p-1 text-muted-foreground hover:text-foreground"
				>
					{entry.masked ? <Eye className="size-3" /> : <EyeOff className="size-3" />}
				</button>
				<button
					type="button"
					onClick={() => {
						setValue(entry.value);
						setEditing(!editing);
					}}
					className="p-1 text-muted-foreground hover:text-foreground"
				>
					<Pencil className="size-3" />
				</button>
				<button
					type="button"
					onClick={onDelete}
					disabled={disabled}
					className="p-1 text-muted-foreground hover:text-destructive"
				>
					<Trash2 className="size-3" />
				</button>
			</div>
		</div>
	);
}
