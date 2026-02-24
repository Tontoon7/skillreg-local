import { type VariantProps, cva } from "class-variance-authority";
import type * as React from "react";

import { cn } from "@/lib/utils";

const badgeVariants = cva(
	"inline-flex items-center justify-center rounded-full border border-transparent px-2 py-0.5 text-xs font-medium w-fit whitespace-nowrap shrink-0 gap-1 transition-[color,box-shadow] overflow-hidden",
	{
		variants: {
			variant: {
				default:
					"bg-primary/20 text-primary border border-primary/30 shadow-[0_0_6px_var(--glow-amber)]",
				secondary: "bg-secondary border border-border text-secondary-foreground",
				destructive:
					"bg-destructive/20 text-destructive border border-destructive/30 shadow-[0_0_6px_var(--glow-red)]",
				outline: "border border-border text-secondary-foreground",
			},
		},
		defaultVariants: {
			variant: "default",
		},
	},
);

function Badge({
	className,
	variant = "default",
	...props
}: React.ComponentProps<"span"> & VariantProps<typeof badgeVariants>) {
	return (
		<span data-slot="badge" className={cn(badgeVariants({ variant }), className)} {...props} />
	);
}

export { Badge, badgeVariants };
