import type * as React from "react";

import { cn } from "@/lib/utils";

function Card({ className, ...props }: React.ComponentProps<"div">) {
	return (
		<div
			data-slot="card"
			className={cn(
				"text-card-foreground flex flex-col gap-6 rounded-xl py-6 panel-inset hover:shadow-[0_0_12px_var(--glow-amber),inset_0_2px_4px_rgba(0,0,0,0.6),inset_0_0_16px_rgba(0,0,0,0.25),0_1px_0_rgba(255,255,255,0.04)] transition-shadow",
				className,
			)}
			{...props}
		/>
	);
}

function CardHeader({ className, ...props }: React.ComponentProps<"div">) {
	return (
		<div
			data-slot="card-header"
			className={cn("grid auto-rows-min items-start gap-2 px-6", className)}
			{...props}
		/>
	);
}

function CardTitle({ className, ...props }: React.ComponentProps<"div">) {
	return (
		<div
			data-slot="card-title"
			className={cn(
				"leading-none font-semibold [font-family:'Chakra_Petch',sans-serif] tracking-wide",
				className,
			)}
			{...props}
		/>
	);
}

function CardDescription({ className, ...props }: React.ComponentProps<"div">) {
	return (
		<div
			data-slot="card-description"
			className={cn("text-muted-foreground text-sm", className)}
			{...props}
		/>
	);
}

function CardContent({ className, ...props }: React.ComponentProps<"div">) {
	return <div data-slot="card-content" className={cn("px-6", className)} {...props} />;
}

function CardFooter({ className, ...props }: React.ComponentProps<"div">) {
	return (
		<div data-slot="card-footer" className={cn("flex items-center px-6", className)} {...props} />
	);
}

export { Card, CardHeader, CardFooter, CardTitle, CardDescription, CardContent };
