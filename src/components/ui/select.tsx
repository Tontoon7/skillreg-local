import type * as React from "react";

import { cn } from "@/lib/utils";

function Select({ className, ...props }: React.ComponentProps<"select">) {
	return (
		<select
			data-slot="select"
			className={cn(
				"border-border bg-input h-9 w-full rounded-md border px-3 py-1 text-sm shadow-[inset_0_1px_3px_rgba(0,0,0,0.4)] outline-none focus-visible:border-primary/50 focus-visible:ring-ring/50 focus-visible:ring-[3px] focus-visible:shadow-[0_0_8px_var(--glow-amber),inset_0_1px_3px_rgba(0,0,0,0.4)] disabled:cursor-not-allowed disabled:opacity-50",
				className,
			)}
			{...props}
		/>
	);
}

export { Select };
