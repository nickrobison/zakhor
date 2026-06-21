import { useEffect, useMemo, useState } from "react";
import "@xyflow/react/dist/style.css";
import { Background, Controls, MarkerType, MiniMap, Position, ReactFlow, useReactFlow, useEdgesState, useNodesState } from "@xyflow/react";
import type { Edge, Node, NodeMouseHandler } from "@xyflow/react";
import { useQuery } from "@tanstack/react-query";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Separator } from "@/components/ui/separator";
import { traverseGraph } from "@/lib/api/graph";

type GraphNodeKind = "start" | "related";
type GraphNodeData = { label: string; uri: string; kind: GraphNodeKind };
type GraphNode = Node<GraphNodeData>;

const DEPTHS = [1, 2, 3] as const;
const START_NODE_ID = "__zakhor_start__";

function labelForUri(uri: string) {
  return uri.replace(/^.*[#/]/, "");
}

function nodeKindForUri(uri: string): GraphNodeKind {
  return uri === START_NODE_ID ? "start" : "related";
}

function entityDetailHref(uri: string) {
  return isEntityUri(uri) ? `/entities/$entityId/${encodeURIComponent(uri)}` : "";
}

function isEntityUri(uri: string) {
  return uri.includes("entity") || uri.includes("Entity");
}

function edgeIdFor(subject: string, predicate: string, object: string) {
  return `${subject}\u0000${predicate}\u0000${object}`;
}

function buildGraph(startId: string, triples: { subject: string; predicate: string; object: string }[]) {
  const adjacency = new Map<string, { predicate: string; target: string }[]>();
  const incoming = new Map<string, { predicate: string; source: string }[]>();
  const uniqueTriples = new Map<string, { subject: string; predicate: string; object: string }>();

  for (const triple of triples) {
    uniqueTriples.set(edgeIdFor(triple.subject, triple.predicate, triple.object), triple);
    adjacency.set(triple.subject, [...(adjacency.get(triple.subject) ?? []), { predicate: triple.predicate, target: triple.object }]);
    incoming.set(triple.object, [...(incoming.get(triple.object) ?? []), { predicate: triple.predicate, source: triple.subject }]);
  }

  const depths = new Map<string, number>([[startId, 0]]);
  const queue = [startId];
  while (queue.length > 0) {
    const current = queue.shift();
    if (!current) break;
    const currentDepth = depths.get(current) ?? 0;
    if (currentDepth >= 3) continue;
    for (const next of adjacency.get(current) ?? []) {
      if (!depths.has(next.target)) {
        depths.set(next.target, currentDepth + 1);
        queue.push(next.target);
      }
    }
    for (const prev of incoming.get(current) ?? []) {
      if (!depths.has(prev.source)) {
        depths.set(prev.source, currentDepth + 1);
        queue.push(prev.source);
      }
    }
  }

  const nodeUris = new Set([startId, ...Array.from(depths.keys())]);
  const nodes: GraphNode[] = Array.from(nodeUris).map((uri) => {
    const depth = depths.get(uri) ?? 0;
    const angle = depth === 0 ? 0 : ((Array.from(depths.values()).filter((value) => value === depth).length % 8) * Math.PI) / 4;
    const radius = depth * 240;
    return {
      id: uri,
      position: { x: Math.cos(angle) * radius, y: Math.sin(angle) * radius },
      sourcePosition: Position.Right,
      targetPosition: Position.Left,
      data: { label: uri === startId ? labelForUri(startId) : labelForUri(uri), uri, kind: nodeKindForUri(uri) },
    };
  });

  const edges: Edge[] = Array.from(uniqueTriples.values()).map((triple, index) => ({
    id: edgeIdFor(triple.subject, triple.predicate, triple.object),
    source: triple.subject,
    target: triple.object,
    label: labelForUri(triple.predicate),
    labelStyle: { fill: "var(--muted-foreground)", fontSize: 11, fontWeight: 600 },
    style: { stroke: "var(--border)", strokeWidth: 1.5 },
    ariaLabel: `${labelForUri(triple.subject)} ${labelForUri(triple.predicate)} ${labelForUri(triple.object)}`,
    markerEnd: index % 2 === 0 ? MarkerType.ArrowClosed : undefined,
  }));

  return { nodes, edges };
}

export function GraphPage() {
  const { fitView } = useReactFlow();
  const [startId, setStartId] = useState("");
  const [draftStartId, setDraftStartId] = useState("");
  const [depth, setDepth] = useState(1);
  const [edgeTypes, setEdgeTypes] = useState("");
  const [selectedUri, setSelectedUri] = useState<string | null>(null);
  const [nodes, setNodes, onNodesChange] = useNodesState<GraphNode>([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState<Edge>([]);

  const edgeTypeList = useMemo(
    () =>
      edgeTypes
        .split(",")
        .map((value) => value.trim())
        .filter(Boolean),
    [edgeTypes],
  );

  const graph = useQuery({
    queryKey: ["graph", startId, depth, edgeTypeList.join("|")],
    queryFn: () => traverseGraph(startId, depth, edgeTypeList),
    enabled: startId.trim().length > 0,
    retry: false,
  });

  const flow = useMemo(() => buildGraph(startId, graph.data?.triples ?? []), [graph.data?.triples, startId]);

  useEffect(() => {
    setNodes(flow.nodes);
    setEdges(flow.edges);
    if (flow.nodes.length > 0) {
      void fitView({ nodes: flow.nodes, padding: 0.18, duration: 150 });
    }
  }, [fitView, flow.edges, flow.nodes, setEdges, setNodes]);

  useEffect(() => {
    if (graph.isError) setSelectedUri(null);
  }, [graph.isError]);

  function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const nextStartId = draftStartId.trim();
    setStartId(nextStartId);
    setSelectedUri(nextStartId || null);
  }

  function handleDepthChange(value: string) {
    const nextDepth = Number(value);
    setDepth(nextDepth);
  }

  const handleNodeClick: NodeMouseHandler<GraphNode> = (_, node) => {
    setSelectedUri(node.id);
  };

  const selectedNode = nodes.find((node) => node.id === selectedUri);
  const relatedEdges = edges.filter((edge) => edge.source === selectedUri || edge.target === selectedUri);
  const warning = graph.data?.warning ?? (depth === 3 ? "Large graph — consider narrowing search." : "");

  return (
    <section className="space-y-6">
      <div>
        <h1 className="text-3xl font-semibold tracking-tight">Graph Explorer</h1>
        <p className="mt-2 text-muted-foreground">Interactive @xyflow/react traversal of Tracker relationships.</p>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Relationship traversal</CardTitle>
          <CardDescription>Enter a node URI, choose a depth from 1 to 3, and optionally filter by edge predicate.</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <form className="flex flex-col gap-3 lg:flex-row lg:items-end" onSubmit={handleSubmit}>
            <div className="flex-1 space-y-2">
              <label className="text-sm font-medium" htmlFor="graph-start-id">
                Start node URI
              </label>
              <Input
                id="graph-start-id"
                value={draftStartId}
                onChange={(event) => setDraftStartId(event.target.value)}
                placeholder="Start node URI"
              />
            </div>
            <div className="space-y-2 sm:w-40">
              <label className="text-sm font-medium" htmlFor="graph-depth">
                Depth
              </label>
              <select
                id="graph-depth"
                value={depth}
                onChange={(event) => handleDepthChange(event.target.value)}
                className="h-9 w-full rounded-md border border-input bg-transparent px-3 text-sm"
              >
                {DEPTHS.map((value) => (
                  <option key={value} value={value}>
                    {value}
                  </option>
                ))}
              </select>
            </div>
            <div className="space-y-2 lg:w-64">
              <label className="text-sm font-medium" htmlFor="graph-edge-types">
                Edge predicates
              </label>
              <Input id="graph-edge-types" value={edgeTypes} onChange={(event) => setEdgeTypes(event.target.value)} placeholder="Optional comma-separated predicates" />
            </div>
            <Button type="submit" disabled={!draftStartId.trim()}>
              Traverse
            </Button>
          </form>

          {warning && <p className="text-sm text-muted-foreground">{warning}</p>}
          {graph.isError && <p className="text-sm text-destructive">Failed to load graph. Ensure the Rust API is running.</p>}
          {graph.isLoading && <p className="text-sm text-muted-foreground">Loading graph…</p>}

          <div className="grid h-[560px] gap-4 lg:grid-cols-[minmax(0,1fr)_320px]">
            <div className="min-h-0 rounded-md border bg-background">
              <ReactFlow
                nodes={nodes}
                edges={edges}
                onNodesChange={onNodesChange}
                onEdgesChange={onEdgesChange}
                onNodeClick={handleNodeClick}
                fitView
                fitViewOptions={{ padding: 0.16 }}
                minZoom={0.1}
                maxZoom={2}
              >
                <Background />
                <MiniMap pannable zoomable />
                <Controls />
              </ReactFlow>
            </div>

            <ScrollArea className="h-full rounded-md border bg-card p-4">
              {selectedNode ? (
                <NodeDetails node={selectedNode} edges={relatedEdges} />
              ) : (
                <EmptyGraphPanel />
              )}
            </ScrollArea>
          </div>
        </CardContent>
      </Card>
    </section>
  );
}

function NodeDetails({ node, edges }: { node: GraphNode; edges: Edge[] }) {
  const href = entityDetailHref(node.data.uri);

  return (
    <div className="space-y-4">
      <div>
        <h2 className="text-lg font-semibold">{node.data.label}</h2>
        <p className="mt-1 break-all text-sm text-muted-foreground">{node.data.uri}</p>
      </div>

      <Badge variant={node.data.kind === "start" ? "default" : "secondary"}>{node.data.kind === "start" ? "Start node" : "Related node"}</Badge>

      <div>
        <p className="text-sm font-medium">Type</p>
        <p className="mt-1 text-sm text-muted-foreground">{inferNodeType(node.data.uri)}</p>
      </div>

      <Separator />

      <div className="space-y-2">
        <p className="text-sm font-medium">Adjacent relationships</p>
        {edges.length > 0 ? (
          <ul className="space-y-2 text-sm text-muted-foreground">
            {edges.map((edge) => (
              <li key={edge.id} className="rounded-md border p-2">
                <span className="font-medium text-foreground">{edge.label}</span>
              </li>
            ))}
          </ul>
        ) : (
          <p className="text-sm text-muted-foreground">No adjacent relationships in the current traversal.</p>
        )}
      </div>

      {href ? (
        <a href={href} className="inline-flex items-center rounded-md bg-primary px-3 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-primary/90">
          Open entity detail
        </a>
      ) : (
        <p className="text-sm text-muted-foreground">This node is not modeled as an entity URI, so no entity detail link is available.</p>
      )}
    </div>
  );
}

function EmptyGraphPanel() {
  return (
    <div className="space-y-4">
      <p className="text-sm font-medium">Node details</p>
      <p className="text-sm text-muted-foreground">Traverse a graph, then click a node to inspect its URI, inferred type, and adjacent relationships.</p>
    </div>
  );
}

function inferNodeType(uri: string) {
  const normalized = uri.toLowerCase();
  if (normalized.includes("entity")) return "Entity";
  if (normalized.includes("observation") || normalized.includes("informationelement")) return "Observation";
  if (normalized.includes("decision") || normalized.includes("outcome")) return "Decision";
  return "Unknown";
}
