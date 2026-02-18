import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Select } from "@/components/ui/select";
import { listSkills, scanLocalSkills } from "@/lib/api";
import { useConfigStore } from "@/lib/store";
import { tagStyle } from "@/lib/tag-colors";
import type { PaginatedSkills, RegistrySkill } from "@/lib/types";
import { cn } from "@/lib/utils";
import {
	Check,
	ChevronLeft,
	ChevronRight,
	Download,
	Loader2,
	Package,
	Search,
	X,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useNavigate } from "react-router";

type SortOption = "updated" | "downloads" | "name";

export function Catalog() {
	const org = useConfigStore((s) => s.config.org);
	const navigate = useNavigate();

	const [data, setData] = useState<PaginatedSkills | null>(null);
	const [loading, setLoading] = useState(true);
	const [error, setError] = useState<string | null>(null);
	const [search, setSearch] = useState("");
	const [debouncedSearch, setDebouncedSearch] = useState("");
	const [sort, setSort] = useState<SortOption>("updated");
	const [page, setPage] = useState(1);
	const [selectedTags, setSelectedTags] = useState<string[]>([]);
	const [knownTags, setKnownTags] = useState<Set<string>>(new Set());
	const [installedNames, setInstalledNames] = useState<Set<string>>(new Set());

	const debounceRef = useRef<ReturnType<typeof setTimeout>>(undefined);

	useEffect(() => {
		scanLocalSkills()
			.then((locals) => {
				const names = new Set<string>();
				for (const s of locals) {
					// Frontmatter name (e.g. "SVG Logo Designer")
					names.add(s.name.toLowerCase());
					// Directory name = registry slug (e.g. "svg-logo-designer")
					const dirName = s.path.split("/").pop() || s.path.split("\\").pop();
					if (dirName) names.add(dirName.toLowerCase());
				}
				setInstalledNames(names);
			})
			.catch(() => {});
	}, []);

	const fetchSkills = useCallback(
		async (s: SortOption) => {
			if (!org) return;
			setLoading(true);
			setError(null);
			try {
				const result = await listSkills({
					org,
					page: 1,
					limit: 200,
					sort: s,
				});
				setData(result);
				const tags = new Set<string>();
				for (const skill of result.skills) {
					for (const tag of skill.tags) tags.add(tag);
				}
				setKnownTags(tags);
			} catch (e) {
				setError(typeof e === "string" ? e : "Failed to load skills");
			} finally {
				setLoading(false);
			}
		},
		[org],
	);

	useEffect(() => {
		fetchSkills(sort);
	}, [fetchSkills, sort]);

	// Client-side search + tag filtering
	const filteredSkills = useMemo(() => {
		if (!data) return [];
		let result = data.skills;

		if (debouncedSearch) {
			const q = debouncedSearch.toLowerCase();
			result = result.filter(
				(s) =>
					s.name.toLowerCase().includes(q) ||
					(s.description?.toLowerCase().includes(q) ?? false) ||
					s.tags.some((t) => t.toLowerCase().includes(q)),
			);
		}

		if (selectedTags.length > 0) {
			result = result.filter((s) => selectedTags.every((tag) => s.tags.includes(tag)));
		}

		return result;
	}, [data, debouncedSearch, selectedTags]);

	const PAGE_SIZE = 20;
	const totalPages = Math.ceil(filteredSkills.length / PAGE_SIZE);
	const pagedSkills = filteredSkills.slice((page - 1) * PAGE_SIZE, page * PAGE_SIZE);

	const handleSearch = (value: string) => {
		setSearch(value);
		clearTimeout(debounceRef.current);
		debounceRef.current = setTimeout(() => {
			setPage(1);
			setDebouncedSearch(value);
		}, 300);
	};

	const toggleTag = (tag: string) => {
		setPage(1);
		setSelectedTags((prev) =>
			prev.includes(tag) ? prev.filter((t) => t !== tag) : [...prev, tag],
		);
	};

	const clearFilters = () => {
		setSearch("");
		setDebouncedSearch("");
		setSelectedTags([]);
		setPage(1);
	};

	const availableTags = useMemo(() => [...knownTags].sort(), [knownTags]);

	const hasActiveFilters = debouncedSearch || selectedTags.length > 0;

	if (!org) {
		return (
			<div className="flex flex-col items-center justify-center gap-3 h-full">
				<Package className="size-10 text-muted-foreground" />
				<p className="text-muted-foreground">Select an organization first</p>
				<Button variant="outline" size="sm" onClick={() => navigate("/settings")}>
					Go to Settings
				</Button>
			</div>
		);
	}

	return (
		<div className="flex flex-col gap-4 p-6">
			<div className="flex items-center justify-between">
				<h1 className="text-lg font-semibold">{org} — Skills</h1>
				{hasActiveFilters && (
					<Button
						variant="ghost"
						size="sm"
						onClick={clearFilters}
						className="text-xs text-muted-foreground"
					>
						<X className="size-3 mr-1" />
						Clear filters
					</Button>
				)}
			</div>

			{/* Search + Sort */}
			<div className="flex items-center gap-3">
				<div className="relative flex-1 min-w-0">
					<Search className="absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
					<Input
						placeholder="Search skills..."
						value={search}
						onChange={(e) => handleSearch(e.target.value)}
						className="pl-9"
					/>
				</div>
				<Select
					value={sort}
					onChange={(e) => {
						setSort(e.target.value as SortOption);
						setPage(1);
					}}
					className="w-auto shrink-0"
				>
					<option value="updated">Recently updated</option>
					<option value="downloads">Most downloads</option>
					<option value="name">Name A-Z</option>
				</Select>
			</div>

			{/* Tag filters */}
			{availableTags.length > 0 && (
				<div className="flex flex-wrap gap-1.5">
					{availableTags.map((tag) => {
						const active = selectedTags.includes(tag);
						return (
							<button
								key={tag}
								type="button"
								onClick={() => toggleTag(tag)}
								className={cn(
									"inline-flex items-center rounded-full border px-2.5 py-0.5 text-xs font-medium transition-all cursor-pointer",
									"hover:brightness-110",
								)}
								style={tagStyle(tag, active)}
							>
								{tag}
								{active && <X className="size-3 ml-1 opacity-70" />}
							</button>
						);
					})}
				</div>
			)}

			{/* Skills list */}
			{loading ? (
				<div className="flex items-center justify-center py-12">
					<Loader2 className="size-6 animate-spin text-muted-foreground" />
				</div>
			) : error ? (
				<div className="flex flex-col items-center justify-center gap-2 py-12">
					<p className="text-sm text-destructive">{error}</p>
					<Button variant="outline" size="sm" onClick={() => fetchSkills(sort)}>
						Retry
					</Button>
				</div>
			) : pagedSkills.length > 0 ? (
				<>
					{hasActiveFilters && (
						<p className="text-xs text-muted-foreground">
							{filteredSkills.length} skill{filteredSkills.length > 1 ? "s" : ""} found
						</p>
					)}

					<div className="space-y-3">
						{pagedSkills.map((skill) => (
							<SkillCard
								key={skill.id}
								skill={skill}
								installed={installedNames.has(skill.name.toLowerCase())}
								selectedTags={selectedTags}
								onTagClick={toggleTag}
								onClick={() => navigate(`/catalog/${skill.name}`)}
							/>
						))}
					</div>

					{/* Pagination */}
					{totalPages > 1 && (
						<div className="flex items-center justify-center gap-2 pt-2">
							<Button
								variant="outline"
								size="sm"
								disabled={page <= 1}
								onClick={() => setPage((p) => p - 1)}
							>
								<ChevronLeft className="size-4" />
							</Button>
							<span className="text-sm text-muted-foreground">
								Page {page} / {totalPages}
							</span>
							<Button
								variant="outline"
								size="sm"
								disabled={page >= totalPages}
								onClick={() => setPage((p) => p + 1)}
							>
								<ChevronRight className="size-4" />
							</Button>
						</div>
					)}
				</>
			) : hasActiveFilters ? (
				<div className="flex flex-col items-center justify-center gap-2 py-12">
					<Package className="size-10 text-muted-foreground" />
					<p className="text-muted-foreground">No skills match your filters</p>
					<Button variant="outline" size="sm" onClick={clearFilters}>
						Clear filters
					</Button>
				</div>
			) : (
				<div className="flex flex-col items-center justify-center gap-2 py-12">
					<Package className="size-10 text-muted-foreground" />
					<p className="text-muted-foreground">No skills found</p>
				</div>
			)}
		</div>
	);
}

function SkillCard({
	skill,
	installed,
	selectedTags,
	onTagClick,
	onClick,
}: {
	skill: RegistrySkill;
	installed: boolean;
	selectedTags: string[];
	onTagClick: (tag: string) => void;
	onClick: () => void;
}) {
	return (
		<button
			type="button"
			onClick={onClick}
			className={cn(
				"flex w-full items-start justify-between rounded-xl border bg-card p-4 text-left transition-colors",
				"hover:border-muted-foreground/30",
			)}
		>
			<div className="flex flex-col gap-1.5 min-w-0 flex-1">
				<div className="flex items-center gap-2">
					<span className="font-medium truncate">{skill.name}</span>
					{skill.latestVersion && <Badge variant="secondary">{skill.latestVersion}</Badge>}
					{installed && (
						<span className="inline-flex items-center gap-1 rounded-full bg-emerald-500/10 px-2 py-0.5 text-[11px] font-medium text-emerald-400 border border-emerald-500/20">
							<Check className="size-3" />
							Installed
						</span>
					)}
				</div>
				{skill.description && (
					<p className="text-sm text-muted-foreground line-clamp-1">{skill.description}</p>
				)}
				{skill.tags.length > 0 && (
					<div className="flex flex-wrap gap-1">
						{skill.tags.slice(0, 5).map((tag) => {
							const active = selectedTags.includes(tag);
							return (
								<button
									key={tag}
									type="button"
									onClick={(e) => {
										e.stopPropagation();
										onTagClick(tag);
									}}
									className="inline-flex items-center rounded-full border px-2 py-0 text-[11px] font-medium transition-all cursor-pointer hover:brightness-125"
									style={tagStyle(tag, active)}
								>
									{tag}
								</button>
							);
						})}
					</div>
				)}
			</div>
			<div className="flex items-center gap-1 text-xs text-muted-foreground shrink-0 ml-4">
				<Download className="size-3" />
				{skill.totalDownloads}
			</div>
		</button>
	);
}
