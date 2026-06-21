import { useQuery } from "@tanstack/react-query";
import { useParams } from "@tanstack/react-router";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Separator } from "@/components/ui/separator";
import { Skeleton } from "@/components/ui/skeleton";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { TooltipProvider } from "@/components/ui/tooltip";
import { getDecision, getDecisionProvenance } from "@/lib/api/decisions";
import type { Edge, Node } from "@xyflow/react";
import { Background, Controls, Position, ReactFlow, useEdgesState, useNodesState } from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import { useCallback, useMemo, useState } from "react";
import { Button } from "@/components/ui/button";
import { ChevronDown, ChevronUp, ExternalLink } from "lucide-react";

type RelNodeData = { label: string; uri: string };
type RelNode = Node<RelNodeData>;

function labelForUri(uri: string) {
  return uri.replace(/^.*[#/]/, "");
}

function buildRelationshipGraph(decisionId: string, relatedIds: string[]) {
  const nodes: RelNode[] = [
    {
      id: decisionId,
      position: { x: 0, y: 0 },
      sourcePosition: Position.Right,
      targetPosition: Position.Left,
      data: { label: "Current decision", uri: decisionId },
    },
  ];

  const edges: Edge[] = [];

  relatedIds.forEach((relatedId, idx) => {
    const angle = (idx - relatedIds.length / 2 + 0.5) * (2 * Math.PI / Math.max(1, relatedIds.length));
    const radius = 200;
    nodes.push({
      id: relatedId,
      position: { x: Math.cos(angle) * radius, y: Math.sin(angle) * radius },
      sourcePosition: Position.Right,
      targetPosition: Position.Left,
      data: { label: labelForUri(relatedId), uri: relatedId },
    });
    edges.push({
      id: `edge-${idx}`,
      source: decisionId,
      target: relatedId,
      label: "related",
      style: { stroke: "var(--border)", strokeWidth: 1.5 },
    });
  });

  return { nodes, edges };
}

export function DecisionDetailPage() {
  const { decisionId } = useParams({ from: "/decisions/$decisionId" });
  const decisionQuery = useQuery({
    queryKey: ["decision", decisionId],
    queryFn: () => getDecision(decisionId),
  });
  const provenanceQuery = useQuery({
    queryKey: ["decision-provenance", decisionId],
    queryFn: () => getDecisionProvenance(decisionId),
  });

  const decision = decisionQuery.data;

  const graph = useMemo(() => {
    if (!decision) return { nodes: [], edges: [] };
    return buildRelationshipGraph(decision.id, decision.related_decision_ids);
  }, [decision]);

  const [nodes] = useNodesState<RelNode>(graph.nodes);
  const [edges] = useEdgesState<Edge>(graph.edges);

  const [isProvenanceExpanded, setIsProvenanceExpanded] = useState(true);
  const [selectedProvenanceStep, setSelectedProvenanceStep] = useState<string | null>(null);

  const onNodeClick = useCallback((_: unknown, node: RelNode) => {
    if (node.id !== decisionId) {
      window.location.href = `/decisions/${encodeURIComponent(node.id)}`;
    }
  }, [decisionId]);

  const handleProvenanceItemClick = useCallback((step: string) => {
    setSelectedProvenanceStep((prev) => (prev === step ? null : step));
  }, []);

  return (
    <TooltipProvider>
      <section className="space-y-6">
        <div>
          <h1 className="text-3xl font-semibold tracking-tight">Decision detail</h1>
          <p className="mt-2 text-muted-foreground">Decision ID: {decisionId}</p>
        </div>

        {decisionQuery.isLoading ? (
          <div className="space-y-4">
            <Skeleton className="h-40 w-full" />
            <Skeleton className="h-80 w-full" />
          </div>
        ) : decisionQuery.isError ? (
          <p className="text-sm text-destructive">Failed to load decision. Ensure the API is running.</p>
        ) : !decision ? (
          <p className="text-sm text-muted-foreground">Decision not found.</p>
        ) : (
          <div className="grid gap-6 lg:grid-cols-[minmax(0,1fr)_360px]">
            <div className="space-y-4">
              <Card>
                <CardHeader>
                  <div className="flex items-center justify-between">
                    <CardTitle>{decision.title}</CardTitle>
                    <Badge>{decision.status}</Badge>
                  </div>
                  <CardDescription className="flex items-center gap-4">
                    {decision.modified && <span>Modified: {new Date(decision.modified).toLocaleDateString()}</span>}
                    {decision.created && <span>Created: {new Date(decision.created).toLocaleDateString()}</span>}
                    {decision.confidence != null && (
                      <span className="text-sm font-medium">{decision.confidence}% confidence</span>
                    )}
                  </CardDescription>
                </CardHeader>
                <CardContent className="space-y-4">
                  {decision.summary && (
                    <div>
                      <h3 className="text-sm font-medium">Summary</h3>
                      <p className="mt-1 text-sm text-muted-foreground">{decision.summary}</p>
                    </div>
                  )}
                  <Separator />
                  <div>
                    <h3 className="text-sm font-medium">Context</h3>
                    <p className="mt-1 text-sm text-muted-foreground">{decision.context}</p>
                  </div>
                  <Separator />
                  <div>
                    <h3 className="text-sm font-medium">Outcome</h3>
                    <p className="mt-1 text-sm text-muted-foreground">{decision.outcome}</p>
                  </div>
                  <Separator />
                  <div>
                    <h3 className="text-sm font-medium">Rationale</h3>
                    <p className="mt-1 text-sm text-muted-foreground">{decision.rationale}</p>
                  </div>
                </CardContent>
              </Card>

              <Tabs defaultValue="evidence">
                <TabsList>
                  <TabsTrigger value="evidence">Evidence</TabsTrigger>
                  <TabsTrigger value="entities">Entities</TabsTrigger>
                  <TabsTrigger value="code">Code impact</TabsTrigger>
                  <TabsTrigger value="related">Related</TabsTrigger>
                  <TabsTrigger value="graph">Graph</TabsTrigger>
                </TabsList>

                <TabsContent value="evidence">
                  <Card>
                    <CardContent className="pt-6 space-y-3">
                      {decision.evidence.length === 0 ? (
                        <p className="text-sm text-muted-foreground">No evidence recorded.</p>
                      ) : (
                        decision.evidence.map((item, idx) => (
                          <div key={idx} className="border-l-2 pl-3">
                            <p className="text-sm font-medium">{item.source}</p>
                            <p className="text-sm text-muted-foreground">{item.content}</p>
                          </div>
                        ))
                      )}
                    </CardContent>
                  </Card>
                </TabsContent>

                <TabsContent value="entities">
                  <Card>
                    <CardContent className="pt-6 space-y-2">
                      {decision.entities.length === 0 ? (
                        <p className="text-sm text-muted-foreground">No entities linked.</p>
                      ) : (
                        decision.entities.map((entity) => (
                          <div key={entity.uri} className="flex items-center gap-2">
                            <Badge variant="outline" className="font-mono text-xs">
                              {entity.uri.split("/").pop()}
                            </Badge>
                            <span className="text-sm">{entity.label}</span>
                          </div>
                        ))
                      )}
                    </CardContent>
                  </Card>
                </TabsContent>

                <TabsContent value="code">
                  <Card>
                    <CardContent className="pt-6 space-y-2">
                      {decision.code_references && decision.code_references.length > 0 ? (
                        decision.code_references.map((ref, idx) => (
                          <div key={idx} className="space-y-1">
                            <p className="text-sm font-medium">{ref.file_path}</p>
                            {ref.repo && <p className="text-xs text-muted-foreground">{ref.repo}</p>}
                          </div>
                        ))
                      ) : (
                        <p className="text-sm text-muted-foreground">No code references.</p>
                      )}
                    </CardContent>
                  </Card>
                </TabsContent>

                <TabsContent value="related">
                  <Card>
                    <CardContent className="pt-6 space-y-2">
                      {decision.related_decision_ids.length === 0 ? (
                        <p className="text-sm text-muted-foreground">No related decisions.</p>
                      ) : (
                        <div className="space-y-2">
                          <p className="text-sm font-medium">Related decisions ({decision.related_decision_ids.length})</p>
                          <ul className="list-disc list-inside text-sm text-muted-foreground">
                            {decision.related_decision_ids.map((id) => (
                              <li key={id}>
                                <a href={`/decisions/${encodeURIComponent(id)}`} className="hover:underline">
                                  {id}
                                </a>
                              </li>
                            ))}
                          </ul>
                        </div>
                      )}
                    </CardContent>
                  </Card>
                </TabsContent>

                <TabsContent value="graph">
                  <Card>
                    <CardContent className="pt-6">
                      {graph.nodes.length <= 1 ? (
                        <p className="text-sm text-muted-foreground">No related decisions to display in graph.</p>
                      ) : (
                        <div className="h-[360px] rounded-md border bg-background">
                          <ReactFlow
                            nodes={nodes}
                            edges={edges}
                            onNodeClick={onNodeClick}
                            fitView
                            fitViewOptions={{ padding: 0.2 }}
                            minZoom={0.5}
                            maxZoom={2}
                          >
                            <Background />
                            <Controls />
                          </ReactFlow>
                        </div>
                      )}
                    </CardContent>
                  </Card>
                </TabsContent>
              </Tabs>
            </div>

            <div>
              <Card>
                <CardHeader className="cursor-pointer select-none" onClick={() => setIsProvenanceExpanded(!isProvenanceExpanded)}>
                  <div className="flex items-center justify-between">
                    <div>
                      <CardTitle>Provenance panel</CardTitle>
                      <CardDescription>Evidence chain for decision {decisionId}</CardDescription>
                    </div>
                    <Button variant="ghost" size="icon" aria-label={isProvenanceExpanded ? "Collapse provenance" : "Expand provenance"}>
                      {isProvenanceExpanded ? <ChevronUp className="h-4 w-4" /> : <ChevronDown className="h-4 w-4" />}
                    </Button>
                  </div>
                </CardHeader>
                {isProvenanceExpanded && (
                  <CardContent>
                    {provenanceQuery.isLoading ? (
                      <div className="space-y-2">
                        <Skeleton className="h-4 w-full" />
                        <Skeleton className="h-4 w-3/4" />
                      </div>
                    ) : provenanceQuery.isError ? (
                      <p className="text-sm text-destructive">Failed to load provenance.</p>
                    ) : provenanceQuery.data?.chain.length === 0 ? (
                      <p className="text-sm text-muted-foreground">No provenance data available.</p>
                    ) : (
                      <ScrollArea className="h-96 rounded-md border p-4">
                        <ol className="space-y-3 text-sm">
                          {provenanceQuery.data?.chain.map((item, idx) => (
                            <li key={idx}>
                              <button
                                type="button"
                                className="w-full rounded-md p-2 text-left transition-colors hover:bg-accent hover:text-accent-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                                onClick={(e) => {
                                  e.stopPropagation();
                                  handleProvenanceItemClick(item.step);
                                }}
                                aria-label={`Provenance step: ${item.step}`}
                              >
                                <span className="font-medium">{item.step}:</span> {item.label}
                                {selectedProvenanceStep === item.step && (
                                  <span className="ml-2 inline-flex items-center gap-1 text-xs text-primary">
                                    <ExternalLink className="h-3 w-3" />
                                    {item.source}
                                  </span>
                                )}
                                <span className="block text-xs text-muted-foreground">{item.source}</span>
                              </button>
                              {selectedProvenanceStep === item.step && (
                                <div className="ml-2 mt-1 rounded-md border bg-muted/50 p-3 text-xs text-muted-foreground">
                                  <p>
                                    <span className="font-medium text-foreground">Step:</span> {item.step}
                                  </p>
                                  <p>
                                    <span className="font-medium text-foreground">Source:</span> {item.source}
                                  </p>
                                </div>
                              )}
                            </li>
                          ))}
                        </ol>
                      </ScrollArea>
                    )}
                  </CardContent>
                )}
              </Card>
            </div>
          </div>
        )}
      </section>
    </TooltipProvider>
  );
}