import { useQuery } from "@tanstack/react-query";
import { Link, useParams } from "@tanstack/react-router";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Skeleton } from "@/components/ui/skeleton";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { getEntity, getEntityDecisions, getEntityObservations } from "@/lib/api/entities";

export function EntityDetailPage() {
  const { entityId } = useParams({ from: "/entities/$entityId" });

  const entityQuery = useQuery({
    queryKey: ["entity", entityId],
    queryFn: () => getEntity(entityId),
  });

  const decisionsQuery = useQuery({
    queryKey: ["entity-decisions", entityId],
    queryFn: () => getEntityDecisions(entityId),
  });

  const observationsQuery = useQuery({
    queryKey: ["entity-observations", entityId],
    queryFn: () => getEntityObservations(entityId),
  });

  const entity = entityQuery.data;

  return (
    <section className="space-y-6">
      <div>
        <h1 className="text-3xl font-semibold tracking-tight">Entity detail</h1>
        <p className="mt-2 text-muted-foreground">Entity ID: {entityId}</p>
      </div>

      {entityQuery.isLoading ? (
        <div className="space-y-4">
          <Skeleton className="h-40 w-full" />
          <Skeleton className="h-80 w-full" />
        </div>
      ) : entityQuery.isError ? (
        <p className="text-sm text-destructive">Failed to load entity. Ensure the API is running.</p>
      ) : !entity ? (
        <p className="text-sm text-muted-foreground">Entity not found.</p>
      ) : (
        <div className="grid gap-6 lg:grid-cols-[minmax(0,1fr)_360px]">
          <div className="space-y-4">
            <Card>
              <CardHeader>
                <div className="flex items-center justify-between">
                  <CardTitle>{entity.label}</CardTitle>
                  <Badge>{entity.uri}</Badge>
                </div>
                <CardDescription>
                  {entity.types.length > 0 ? (
                    <span className="flex flex-wrap gap-1 mt-2">
                      {entity.types.map((type) => (
                        <Badge key={type} variant="secondary">
                          {type}
                        </Badge>
                      ))}
                    </span>
                  ) : null}
                </CardDescription>
              </CardHeader>
            </Card>

            <Tabs defaultValue="relationships">
              <TabsList>
                <TabsTrigger value="relationships">Relationships</TabsTrigger>
                <TabsTrigger value="decisions">Decisions</TabsTrigger>
                <TabsTrigger value="observations">Observations</TabsTrigger>
              </TabsList>

              <TabsContent value="relationships">
                <Card>
                  <CardContent className="pt-6">
                    {entity.relationships.length === 0 ? (
                      <p className="text-sm text-muted-foreground">No relationships found.</p>
                    ) : (
                      <ScrollArea className="h-80 rounded-md border">
                        <Table>
                          <TableHeader>
                            <TableRow>
                              <TableHead>Subject</TableHead>
                              <TableHead>Predicate</TableHead>
                              <TableHead>Object</TableHead>
                            </TableRow>
                          </TableHeader>
                          <TableBody>
                            {entity.relationships.map((rel, idx) => (
                              <TableRow key={idx}>
                                <TableCell className="font-mono text-xs">{rel.subject_uri}</TableCell>
                                <TableCell>{rel.label}</TableCell>
                                <TableCell className="font-mono text-xs">{rel.object_uri}</TableCell>
                              </TableRow>
                            ))}
                          </TableBody>
                        </Table>
                      </ScrollArea>
                    )}
                  </CardContent>
                </Card>
              </TabsContent>

              <TabsContent value="decisions">
                <Card>
                  <CardContent className="pt-6">
                    {decisionsQuery.isLoading ? (
                      <div className="space-y-2">
                        <Skeleton className="h-4 w-full" />
                        <Skeleton className="h-4 w-3/4" />
                      </div>
                    ) : decisionsQuery.isError ? (
                      <p className="text-sm text-destructive">Failed to load decisions.</p>
                    ) : decisionsQuery.data?.decisions.length === 0 ? (
                      <p className="text-sm text-muted-foreground">No related decisions.</p>
                    ) : (
                      <div className="space-y-2">
                        {decisionsQuery.data?.decisions.map((decision) => (
                          <Link
                            key={decision.id}
                            to="/decisions/$decisionId"
                            params={{ decisionId: decision.id }}
                            className="block hover:bg-accent p-2 rounded"
                          >
                            <div className="flex items-center justify-between">
                              <span className="text-sm font-medium">{decision.title}</span>
                              <Badge variant="outline">{decision.status}</Badge>
                            </div>
                          </Link>
                        ))}
                      </div>
                    )}
                  </CardContent>
                </Card>
              </TabsContent>

              <TabsContent value="observations">
                <Card>
                  <CardContent className="pt-6">
                    {observationsQuery.isLoading ? (
                      <div className="space-y-2">
                        <Skeleton className="h-4 w-full" />
                        <Skeleton className="h-4 w-3/4" />
                      </div>
                    ) : observationsQuery.isError ? (
                      <p className="text-sm text-destructive">Failed to load observations.</p>
                    ) : observationsQuery.data?.observations.length === 0 ? (
                      <p className="text-sm text-muted-foreground">No related observations.</p>
                    ) : (
                      <ScrollArea className="h-80 rounded-md border">
                        <div className="p-4 space-y-3">
                          {observationsQuery.data?.observations.map((obs) => (
                            <div key={obs.uri} className="border-l-2 pl-3">
                              <p className="text-sm font-mono">{obs.uri}</p>
                              <p className="text-sm text-muted-foreground">{obs.text}</p>
                            </div>
                          ))}
                        </div>
                      </ScrollArea>
                    )}
                  </CardContent>
                </Card>
              </TabsContent>
            </Tabs>
          </div>

          <Card>
            <CardHeader>
              <CardTitle>Source locations</CardTitle>
              <CardDescription>Files and locations referencing this entity</CardDescription>
            </CardHeader>
            <CardContent>
              {entity.source_locations.length === 0 ? (
                <p className="text-sm text-muted-foreground">No source locations.</p>
              ) : (
                <ScrollArea className="h-80 rounded-md border p-4">
                  <ul className="space-y-2 text-sm">
                    {entity.source_locations.map((loc) => (
                      <li key={loc.uri}>
                        <span className="font-medium">{loc.label}</span>
                        <span className="block text-xs font-mono text-muted-foreground">{loc.uri}</span>
                      </li>
                    ))}
                  </ul>
                </ScrollArea>
              )}
            </CardContent>
          </Card>
        </div>
      )}
    </section>
  );
}
