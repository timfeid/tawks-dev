import { Tabs as TabsPrimitive } from "bits-ui";
import Content from "./tabs-content.svelte";
import List from "./tabs-list.svelte";
import Trigger from "./tabs-trigger.svelte";
import Count from "./tabs-count.svelte";
import TabBar, { type TabBarItem } from "./tab-bar.svelte";

const Root = TabsPrimitive.Root;

export {
	Root,
	Content,
	List,
	Trigger,
	Count,
	TabBar,
	type TabBarItem,
	//
	Root as Tabs,
	Content as TabsContent,
	List as TabsList,
	Trigger as TabsTrigger,
	Count as TabsCount,
};
export { tabsLook, triggerLook } from "./look.js";
