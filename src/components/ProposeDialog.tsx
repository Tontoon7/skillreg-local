import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { proposeSkillChange } from "@/lib/api";
import { notify } from "@/lib/notifications";
import type { LocalSkill, ProposalSummary } from "@/lib/types";
import { FilePlus2, Loader2, Send, X } from "lucide-react";
import { useEffect, useState } from "react";

interface ProposeDialogProps {
	skill: LocalSkill;
	org: string;
	onClose: () => void;
	onSubmitted: (proposal: ProposalSummary) => void;
}

export function ProposeDialog({ skill, org, onClose, onSubmitted }: ProposeDialogProps) {
	const [title, setTitle] = useState("");
	const [intent, setIntent] = useState("");
	const [loading, setLoading] = useState(false);
	const [error, setError] = useState<string | null>(null);

	useEffect(() => {
		const handler = (event: KeyboardEvent) => {
			if (event.key === "Escape") onClose();
		};
		window.addEventListener("keydown", handler);
		return () => window.removeEventListener("keydown", handler);
	}, [onClose]);

	const handleSubmit = async () => {
		setLoading(true);
		setError(null);

		try {
			const proposal = await proposeSkillChange({
				org,
				dirPath: skill.path,
				title,
				intent,
			});
			onSubmitted(proposal);
			notify("Proposal submitted", `${skill.name} (${proposal.baseVersion})`);
			onClose();
		} catch (event) {
			const message = typeof event === "string" ? event : "Proposal submission failed";
			setError(message);
			notify("Proposal failed", message);
		} finally {
			setLoading(false);
		}
	};

	return (
		<div className="fixed inset-0 z-50 flex items-center justify-center">
			<div className="absolute inset-0 bg-black/50" onClick={onClose} onKeyDown={undefined} />

			<div className="relative z-10 w-full max-w-lg rounded-xl border bg-card p-6 shadow-lg space-y-5">
				<div className="flex items-center justify-between">
					<div className="space-y-1">
						<h2 className="text-base font-semibold">Propose a change</h2>
						<p className="text-sm text-muted-foreground">
							Submit this local `SKILL.md` as a proposal for review.
						</p>
					</div>
					<Button variant="ghost" size="icon" onClick={onClose} className="size-7">
						<X className="size-4" />
					</Button>
				</div>

				<div className="rounded-lg border p-3 space-y-2">
					<div className="flex items-center gap-2">
						<FilePlus2 className="size-4 text-primary" />
						<span className="text-sm font-medium">{skill.name}</span>
						<Badge variant="secondary">{skill.version}</Badge>
					</div>
					<p className="text-xs text-muted-foreground/70 font-mono break-all">{skill.path}</p>
					<p className="text-xs text-muted-foreground">
						Submitting to <span className="font-medium text-foreground">@{org}</span>
					</p>
				</div>

				<div className="space-y-2">
					<Label htmlFor="proposal-title">Title</Label>
					<Input
						id="proposal-title"
						value={title}
						onChange={(event) => setTitle(event.target.value)}
						placeholder="Summarize the proposed change"
					/>
				</div>

				<div className="space-y-2">
					<Label htmlFor="proposal-intent">Intent</Label>
					<textarea
						id="proposal-intent"
						value={intent}
						onChange={(event) => setIntent(event.target.value)}
						placeholder="Explain why this change should be reviewed"
						rows={5}
						className="w-full rounded-md border bg-background px-3 py-2 text-sm outline-none transition-colors focus:border-primary"
					/>
				</div>

				{error && (
					<div className="rounded-lg border border-destructive/30 bg-destructive/5 p-3">
						<p className="text-sm text-destructive">{error}</p>
					</div>
				)}

				<div className="flex gap-3">
					<Button variant="outline" onClick={onClose} disabled={loading}>
						Cancel
					</Button>
					<Button
						onClick={handleSubmit}
						disabled={loading || title.trim().length < 3 || intent.trim().length < 3}
					>
						{loading ? (
							<Loader2 className="size-4 animate-spin" />
						) : (
							<Send className="size-4" />
						)}
						Submit proposal
					</Button>
				</div>
			</div>
		</div>
	);
}
