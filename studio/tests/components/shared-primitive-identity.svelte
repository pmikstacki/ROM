<script lang="ts">
  import * as controls from "../../src/controls.ts";
  import * as sharedControls from "rom-ui/controls";
  import * as compositions from "../../src/ui-components.ts";
  import { ConversationLayout as SharedConversationLayout } from "rom-ui/ui/components";
  import * as localalert from "../../src/lib/components/ui/alert/index.js";
  import * as sharedalert from "rom-ui/primitives/alert";
  import * as localalertDialog from "../../src/lib/components/ui/alert-dialog/index.js";
  import * as sharedalertDialog from "rom-ui/primitives/alert-dialog";
  import * as localbadge from "../../src/lib/components/ui/badge/index.js";
  import * as sharedbadge from "rom-ui/primitives/badge";
  import * as localbreadcrumb from "../../src/lib/components/ui/breadcrumb/index.js";
  import * as sharedbreadcrumb from "rom-ui/primitives/breadcrumb";
  import * as localcard from "../../src/lib/components/ui/card/index.js";
  import * as sharedcard from "rom-ui/primitives/card";
  import * as localdropdownMenu from "../../src/lib/components/ui/dropdown-menu/index.js";
  import * as shareddropdownMenu from "rom-ui/primitives/dropdown-menu";
  import * as localselect from "../../src/lib/components/ui/select/index.js";
  import * as sharedselect from "rom-ui/primitives/select";
  import * as localseparator from "../../src/lib/components/ui/separator/index.js";
  import * as sharedseparator from "rom-ui/primitives/separator";
  import * as localsidebar from "../../src/lib/components/ui/sidebar/index.js";
  import * as sharedsidebar from "rom-ui/primitives/sidebar";
  import * as localswitch from "../../src/lib/components/ui/switch/index.js";
  import * as sharedswitch from "rom-ui/primitives/switch";
  import * as localtable from "../../src/lib/components/ui/table/index.js";
  import * as sharedtable from "rom-ui/primitives/table";
  import * as localtabs from "../../src/lib/components/ui/tabs/index.js";
  import * as sharedtabs from "rom-ui/primitives/tabs";
  import * as localtooltip from "../../src/lib/components/ui/tooltip/index.js";
  import * as sharedtooltip from "rom-ui/primitives/tooltip";
  import * as localskeleton from "../../src/lib/components/ui/skeleton/index.js";
  import * as sharedskeleton from "rom-ui/primitives/skeleton";
  const identity = {
    "alert": localalert.Root === sharedalert.Root,
    "alert-dialog": localalertDialog.Root === sharedalertDialog.Root,
    "badge": localbadge.Badge === sharedbadge.Badge,
    "breadcrumb": localbreadcrumb.Root === sharedbreadcrumb.Root,
    "card": localcard.Root === sharedcard.Root,
    "dropdown-menu": localdropdownMenu.Root === shareddropdownMenu.Root,
    "select": localselect.Root === sharedselect.Root,
    "separator": localseparator.Separator === sharedseparator.Separator,
    "sidebar": localsidebar.Root === sharedsidebar.Root,
    "switch": localswitch.Switch === sharedswitch.Switch,
    "table": localtable.Root === sharedtable.Root,
    "tabs": localtabs.Root === sharedtabs.Root,
    "tooltip": localtooltip.Root === sharedtooltip.Root,
    "skeleton": localskeleton.Skeleton === sharedskeleton.Skeleton,
    publicBadge: controls.Badge === sharedControls.Badge,
    conversation: compositions.ConversationLayout === SharedConversationLayout,
  };
</script>
<output data-testid="shared-identity">{JSON.stringify(identity)}</output>
