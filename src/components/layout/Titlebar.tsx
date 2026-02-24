import { getCurrentWindow } from "@tauri-apps/api/window";
import { Minus, Square, X } from "lucide-react";

const appWindow = getCurrentWindow();
const IS_MACOS = navigator.userAgent.includes("Macintosh");

export function Titlebar() {
	if (IS_MACOS) {
		// Native traffic lights via titleBarStyle: "overlay"
		return (
			<div
				data-tauri-drag-region
				className="flex h-[38px] shrink-0 items-center brushed-metal channel-border-b select-none"
			>
				<div data-tauri-drag-region className="flex-1 pl-[78px]" />
			</div>
		);
	}

	// Windows/Linux: custom titlebar with window controls
	return (
		<div
			data-tauri-drag-region
			className="flex h-9 shrink-0 items-center justify-between brushed-metal channel-border-b select-none"
		>
			<div data-tauri-drag-region className="flex-1 pl-4" />
			<div className="flex">
				<button
					type="button"
					onClick={() => appWindow.minimize()}
					className="inline-flex size-9 items-center justify-center text-muted-foreground transition-colors hover:bg-secondary"
				>
					<Minus className="size-3.5" />
				</button>
				<button
					type="button"
					onClick={() => appWindow.toggleMaximize()}
					className="inline-flex size-9 items-center justify-center text-muted-foreground transition-colors hover:bg-secondary"
				>
					<Square className="size-3" />
				</button>
				<button
					type="button"
					onClick={() => appWindow.close()}
					className="inline-flex size-9 items-center justify-center text-muted-foreground transition-colors hover:bg-destructive hover:text-white"
				>
					<X className="size-3.5" />
				</button>
			</div>
		</div>
	);
}
