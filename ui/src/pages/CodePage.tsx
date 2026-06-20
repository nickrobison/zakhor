import { useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { getCodeReferences, type CodeResponse } from "@/lib/api/code";

type CodeGroup = {
  repository: string;
  repositories: CodeResponse["repositories"];
  files: CodeResponse["files"];
  symbols: CodeResponse["symbols"];
};

export function CodePage() {
  const [draftQuery, setDraftQuery] = useState("");
  const [activeQuery, setActiveQuery] = useState("");

  const codeQuery = useQuery({
    queryKey: ["code", activeQuery],
    queryFn: () => getCodeReferences(activeQuery),
    enabled: activeQuery.trim().length > 0,
    retry: false,
  });

  const grouped = useMemo(() => groupCodeReferences(codeQuery.data), [codeQuery.data]);

  function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setActiveQuery(draftQuery.trim());
  }

  return (
    <section className="space-y-6">
      <div>
        <h1 className="text-3xl font-semibold tracking-tight">Code Integration</h1>
        <p className="mt-2 text-muted-foreground">Repositories, files, and symbols linked to entities.</p>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Find code references</CardTitle>
          <CardDescription>Search by entity ID or free-form query, then review repository, file, and symbol groups.</CardDescription>
        </CardHeader>
        <CardContent>
          <form className="flex flex-col gap-3 sm:flex-row" onSubmit={handleSubmit}>
            <Input
              value={draftQuery}
              onChange={(event) => setDraftQuery(event.target.value)}
              placeholder="Entity ID or search query"
              className="flex-1"
              aria-label="Entity ID or search query"
            />
            <Button type="submit" disabled={!draftQuery.trim()}>
              Search
            </Button>
          </form>
        </CardContent>
      </Card>

      {codeQuery.isLoading ? (
        <CodeLoadingState />
      ) : codeQuery.isError ? (
        <CodeErrorState />
      ) : activeQuery ? (
        <CodeResults grouped={grouped} />
      ) : (
        <EmptyCodeState />
      )}
    </section>
  );
}

function CodeResults({ grouped }: { grouped: CodeGroup[] }) {
  if (grouped.length === 0) {
    return <EmptyCodeState query="this query" />;
  }

  return (
    <Tabs defaultValue="repo">
      <TabsList>
        <TabsTrigger value="repo">Repositories</TabsTrigger>
        <TabsTrigger value="file">Files</TabsTrigger>
        <TabsTrigger value="symbol">Symbols</TabsTrigger>
      </TabsList>

      <TabsContent value="repo">
        <Card>
          <CardHeader>
            <CardTitle>Repositories</CardTitle>
            <CardDescription>Grouped by repository.</CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            {grouped.map((group) => (
              <RepositoryGroup key={group.repository} group={group} />
            ))}
          </CardContent>
        </Card>
      </TabsContent>

      <TabsContent value="file">
        <Card>
          <CardHeader>
            <CardTitle>Files</CardTitle>
            <CardDescription>Files related to the query.</CardDescription>
          </CardHeader>
          <CardContent>
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>File</TableHead>
                  <TableHead>Repository</TableHead>
                  <TableHead>Language</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {grouped.flatMap((group) => group.files).map((file) => (
                  <TableRow key={`${file.repository ?? "unknown"}:${file.path}`}>
                    <TableCell className="break-all font-medium">{file.path}</TableCell>
                    <TableCell>{file.repository ?? "Unknown"}</TableCell>
                    <TableCell>{file.language ?? "Unknown"}</TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </CardContent>
        </Card>
      </TabsContent>

      <TabsContent value="symbol">
        <Card>
          <CardHeader>
            <CardTitle>Symbols</CardTitle>
            <CardDescription>Symbols related to the query.</CardDescription>
          </CardHeader>
          <CardContent>
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Name</TableHead>
                  <TableHead>Kind</TableHead>
                  <TableHead>File</TableHead>
                  <TableHead>Line</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {grouped.flatMap((group) => group.symbols).map((symbol) => (
                  <TableRow key={`${symbol.file_path}:${symbol.line}:${symbol.name}`}>
                    <TableCell className="font-medium">{symbol.name}</TableCell>
                    <TableCell>
                      <Badge variant="secondary">{symbol.kind}</Badge>
                    </TableCell>
                    <TableCell className="break-all">{symbol.file_path}</TableCell>
                    <TableCell>{symbol.line}</TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </CardContent>
        </Card>
      </TabsContent>
    </Tabs>
  );
}

function RepositoryGroup({ group }: { group: CodeGroup }) {
  return (
    <article className="space-y-3 rounded-md border p-4">
      <div className="flex flex-wrap items-center gap-2">
        <h3 className="text-base font-semibold">{group.repository}</h3>
        <Badge variant="outline">{group.files.length} files</Badge>
        <Badge variant="outline">{group.symbols.length} symbols</Badge>
      </div>

      {group.repositories.map((repository) => (
        <p key={repository.name} className="text-sm text-muted-foreground">
          {repository.description ?? "No description available."}{" "}
          {repository.url ? (
            <a href={repository.url} target="_blank" rel="noreferrer" className="font-medium text-foreground underline-offset-4 hover:underline">
              Open repository
            </a>
          ) : null}
        </p>
      ))}

      {group.files.length > 0 ? (
        <ul className="space-y-1 text-sm text-muted-foreground">
          {group.files.slice(0, 8).map((file) => (
            <li key={`${file.repository}:${file.path}`} className="break-all">
              {file.path}
            </li>
          ))}
        </ul>
      ) : null}

      {group.symbols.length > 0 ? (
        <div className="flex flex-wrap gap-2">
          {group.symbols.slice(0, 8).map((symbol) => (
            <Badge key={`${symbol.file_path}:${symbol.line}:${symbol.name}`} variant="secondary">
              {symbol.name}
            </Badge>
          ))}
        </div>
      ) : null}
    </article>
  );
}

function groupCodeReferences(data: CodeResponse | undefined): CodeGroup[] {
  if (!data) return [];

  const groups = new Map<string, CodeGroup>();
  const fallbackRepository = "Unknown repository";

  for (const repository of data.repositories) {
    const group = groups.get(repository.name) ?? createGroup(repository.name);
    group.repositories.push(repository);
    groups.set(repository.name, group);
  }

  for (const file of data.files) {
    const repository = file.repository || fallbackRepository;
    const group = groups.get(repository) ?? createGroup(repository);
    group.files.push(file);
    groups.set(repository, group);
  }

  for (const symbol of data.symbols) {
    const repository = inferRepository(symbol.file_path, data);
    const group = groups.get(repository) ?? createGroup(repository);
    group.symbols.push(symbol);
    groups.set(repository, group);
  }

  return Array.from(groups.values()).sort((left, right) => left.repository.localeCompare(right.repository));
}

function createGroup(repository: string): CodeGroup {
  return { repository, repositories: [], files: [], symbols: [] };
}

function inferRepository(filePath: string, data: CodeResponse) {
  const normalizedPath = filePath.replaceAll("\\", "/");
  const matchingRepository = data.repositories.find((repository) => normalizedPath.startsWith(`${repository.name}/`));
  return matchingRepository?.name ?? data.files.find((file) => normalizedPath === file.path)?.repository ?? "Unknown repository";
}

function CodeLoadingState() {
  return (
    <div className="space-y-3">
      <Skeleton className="h-24 w-full" />
      <Skeleton className="h-24 w-full" />
      <Skeleton className="h-24 w-full" />
    </div>
  );
}

function CodeErrorState() {
  return (
    <Card>
      <CardContent className="pt-6">
        <p className="text-sm text-destructive">Failed to load code references. Ensure the Rust API is running.</p>
      </CardContent>
    </Card>
  );
}

function EmptyCodeState({ query = "an entity ID or search query" }: { query?: string }) {
  return (
    <Card>
      <CardContent className="pt-6">
        <p className="text-sm text-muted-foreground">No code references found for {query}. Code indexing is not implemented yet, so this page currently shows API-shaped empty states.</p>
      </CardContent>
    </Card>
  );
}
