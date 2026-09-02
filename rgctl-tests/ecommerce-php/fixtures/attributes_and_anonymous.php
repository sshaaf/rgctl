<?php
namespace App\Fixtures;

#[Route('/api/v1')]
class ApiController
{
    public function factory(): object
    {
        return new class {
            public function handle(): string
            {
                return 'ok';
            }
        };
    }
}
