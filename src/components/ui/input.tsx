import type * as React from "react";

import { cn } from "@/lib/utils";

function Input({ className, type, ...props }: React.ComponentProps<"input">) {
	return (
		<input
			type={type}
			data-slot="input"
			className={cn(
				"placeholder:text-muted-foreground border-border h-9 w-full min-w-0 rounded-md border bg-input px-3 py-1 text-base shadow-[inset_0_1px_3px_rgba(0,0,0,0.4)] transition-[color,box-shadow] outline-none disabled:pointer-events-none disabled:cursor-not-allowed disabled:opacity-50 md:text-sm",
				"focus-visible:border-primary/50 focus-visible:ring-ring/50 focus-visible:ring-[3px] focus-visible:shadow-[0_0_8px_var(--glow-amber),inset_0_1px_3px_rgba(0,0,0,0.4)]",
				className,
			)}
			{...props}
		/>
	);
}

export { Input };
