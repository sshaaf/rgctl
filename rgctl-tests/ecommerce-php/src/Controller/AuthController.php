<?php
namespace App\Controller;

use App\Service\AuthService;

#[Route('/auth')]
class AuthController
{
    public function __construct(private AuthService $authService) {}

    public function login(): ?array
    {
        $email = $_POST['email'] ?? '';
        return $this->authService->login($email);
    }

    public function createHandler(): object
    {
        return new class {
            public function handle(): string
            {
                return 'ok';
            }
        };
    }
}
