import { type VariantProps, cva } from "class-variance-authority";
import type * as React from "react";

import { cn } from "@/lib/utils";

const buttonVariants = cva(
	"inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md text-sm [font-family:'Chakra_Petch',sans-serif] tracking-wide font-semibold transition-all disabled:pointer-events-none disabled:opacity-50 [&_svg]:pointer-events-none [&_svg:not([class*='size-'])]:size-4 shrink-0 [&_svg]:shrink-0 outline-none focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:ring-[3px]",
	{
		variants: {
			variant: {
				default:
					"bg-gradient-to-b from-primary to-primary/85 text-primary-foreground shadow-[inset_0_1px_0_rgba(255,255,255,0.15)] hover:shadow-[0_0_12px_var(--glow-amber),inset_0_1px_0_rgba(255,255,255,0.15)]",
				destructive:
					"bg-gradient-to-b from-destructive to-destructive/85 text-white hover:shadow-[0_0_12px_var(--glow-red)]",
				outline:
					"border border-border bg-transparent hover:shadow-[0_0_8px_var(--glow-amber)] hover:border-primary/50",
				secondary:
					"bg-[linear-gradient(180deg,#182844_0%,#101d35_100%)] border border-[#243a5c] shadow-[0_2px_4px_rgba(0,0,0,0.5),inset_0_0_0_1px_rgba(255,255,255,0.04),inset_0_1px_0_rgba(255,255,255,0.08)] text-secondary-foreground hover:brightness-110",
				ghost: "hover:bg-secondary hover:text-foreground",
				link: "text-primary underline-offset-4 hover:underline",
			},
			size: {
				default: "h-9 px-4 py-2 has-[>svg]:px-3",
				sm: "h-8 rounded-md gap-1.5 px-3 has-[>svg]:px-2.5",
				lg: "h-10 rounded-md px-6 has-[>svg]:px-4",
				icon: "size-9",
			},
		},
		defaultVariants: {
			variant: "default",
			size: "default",
		},
	},
);

function Button({
	className,
	variant = "default",
	size = "default",
	...props
}: React.ComponentProps<"button"> & VariantProps<typeof buttonVariants>) {
	return (
		<button
			data-slot="button"
			className={cn(buttonVariants({ variant, size, className }))}
			{...props}
		/>
	);
}

export { Button, buttonVariants };
